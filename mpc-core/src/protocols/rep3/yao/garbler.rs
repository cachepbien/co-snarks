//! Garbler
//!
//! This module contains the implementation of the garbler for the replicated 3-party garbled circuits as described in [ABY3](https://eprint.iacr.org/2018/403.pdf). Thereby, the whole garbled circuit is buffered before given to the network.
//!
//! This implementation is heavily inspired by [fancy-garbling](https://github.com/GaloisInc/swanky/blob/dev/fancy-garbling/src/garble/garbler.rs)

use super::{GCInputs, GCUtils};
use crate::{
    protocols::rep3::{
        id::PartyID,
        network::{IoContext, Rep3Network},
        IoResult,
    },
    RngType,
};
use ark_ff::PrimeField;
use core::panic;
use fancy_garbling::{
    errors::GarblerError, util::output_tweak, BinaryBundle, Fancy, FancyBinary, WireLabel, WireMod2,
};
use rand::SeedableRng;
use scuttlebutt::Block;
use sha3::{Digest, Sha3_256};

/// This struct implements the garbler for replicated 3-party garbled circuits as described in [ABY3](https://eprint.iacr.org/2018/403.pdf).
pub struct Rep3Garbler<'a, N: Rep3Network> {
    io_context: &'a mut IoContext<N>,
    pub(crate) delta: WireMod2,
    current_output: usize,
    current_gate: usize,
    pub(crate) rng: RngType,
    hash: Sha3_256, // For the ID2 to match everything sent with one hash
    circuit: Vec<[u8; 16]>,
}

impl<'a, N: Rep3Network> Rep3Garbler<'a, N> {
    /// Create a new garbler.
    pub fn new(io_context: &'a mut IoContext<N>) -> Self {
        let mut res = Self::new_with_delta(io_context, WireMod2::default());
        res.delta = GCUtils::random_delta(&mut res.rng);
        res
    }

    /// Create a new garbler with existing delta.
    pub fn new_with_delta(io_context: &'a mut IoContext<N>, delta: WireMod2) -> Self {
        let id = io_context.id;
        let seed = io_context.rngs.generate_garbler_randomness(id);
        let rng = RngType::from_seed(seed);

        Self {
            io_context,
            delta,
            current_output: 0,
            current_gate: 0,
            rng,
            hash: Sha3_256::default(),
            circuit: Vec::new(),
        }
    }

    /// Add the gate to the circuit
    fn add_block_to_circuit(&mut self, block: &Block) {
        match self.io_context.id {
            PartyID::ID0 => {
                panic!("Garbler should not be PartyID::ID0");
            }
            PartyID::ID1 => {
                let mut gate = [0; 16];
                gate.copy_from_slice(block.as_ref());
                self.circuit.push(gate);
            }
            PartyID::ID2 => {
                self.hash.update(block.as_ref());
            }
        }
    }

    /// Sends the circuit to the evaluator
    pub fn send_circuit(&mut self) -> IoResult<()> {
        match self.io_context.id {
            PartyID::ID0 => {
                panic!("Garbler should not be PartyID::ID0");
            }
            PartyID::ID1 => {
                tracing::debug!("Sending circuit ID1");
                // Send the prepared circuit over the network to the evaluator
                let mut empty_circuit = Vec::new();
                std::mem::swap(&mut empty_circuit, &mut self.circuit);
                self.io_context
                    .network
                    .send_many(PartyID::ID0, &empty_circuit)?;
                tracing::debug!("Sending circuit ID1 done");
            }
            PartyID::ID2 => {
                tracing::debug!("Sending circuit ID2");
                // Send the hash of the circuit to the evaluator
                let mut hash = Sha3_256::default();
                std::mem::swap(&mut hash, &mut self.hash);
                let digest = hash.finalize();
                self.io_context
                    .network
                    .send(PartyID::ID0, digest.as_slice())?;
                tracing::debug!("Sending circuit ID2 done");
            }
        }
        Ok(())
    }

    /// This puts the X_0 values into garbler_wires and X_c values into evaluator_wires
    pub fn encode_field<F: PrimeField>(&mut self, field: F) -> GCInputs<WireMod2> {
        GCUtils::encode_field(field, &mut self.rng, self.delta)
    }

    /// Consumes the Garbler and returns the delta.
    pub fn into_delta(self) -> WireMod2 {
        self.delta
    }

    /// The current non-free gate index of the garbling computation
    fn current_gate(&mut self) -> usize {
        let current = self.current_gate;
        self.current_gate += 1;
        current
    }

    /// The current output index of the garbling computation.
    fn current_output(&mut self) -> usize {
        let current = self.current_output;
        self.current_output += 1;
        current
    }

    /// Outputs the values to the evaluator.
    fn output_evaluator(&mut self, x: &[WireMod2]) -> IoResult<()> {
        self.outputs(x).or(Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Output failed",
        )))?;
        Ok(())
    }

    /// Outputs the values to the garbler.
    fn output_garbler(&mut self, x: &[WireMod2]) -> IoResult<Vec<bool>> {
        let blocks = self.read_blocks()?;
        if blocks.len() != x.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid number of blocks received",
            ));
        }

        let mut result = Vec::with_capacity(x.len());
        for (block, zero) in blocks.into_iter().zip(x.iter()) {
            if block == zero.as_block() {
                result.push(false);
            } else if block == zero.plus(&self.delta).as_block() {
                result.push(true);
            } else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid block received",
                ));
            }
        }
        Ok(result)
    }

    /// Outputs the value to all parties
    pub fn output_all_parties(&mut self, x: &[WireMod2]) -> IoResult<Vec<bool>> {
        // Garbler's to evaluator
        self.output_evaluator(x)?;

        // Check consistency with the second garbled circuit before receiving the result
        self.send_circuit()?;

        // Evaluator to garbler
        self.output_garbler(x)
    }

    /// Outputs the value to parties ID0 and ID1
    pub fn output_to_id0_and_id1(&mut self, x: &[WireMod2]) -> IoResult<Option<Vec<bool>>> {
        // Garbler's to evaluator
        self.output_evaluator(x)?;

        // Check consistency with the second garbled circuit before receiving the result
        self.send_circuit()?;

        // Evaluator to garbler
        if self.io_context.id == PartyID::ID1 {
            Ok(Some(self.output_garbler(x)?))
        } else {
            Ok(None)
        }
    }

    // Read `Block`s from the channel.
    #[inline(always)]
    fn read_blocks(&mut self) -> IoResult<Vec<Block>> {
        let rcv: Vec<[u8; 16]> = self.io_context.network.recv_many(PartyID::ID0)?;
        let mut result = Vec::with_capacity(rcv.len());
        for block in rcv {
            let mut v = Block::default();
            v.as_mut().copy_from_slice(&block);
            result.push(v);
        }
        Ok(result)
    }

    /// Send a wire over the established channel.
    fn add_wire_to_circuit(&mut self, wire: &WireMod2) {
        self.add_block_to_circuit(&wire.as_block());
    }

    /// Send a bundle of wires over the established channel.
    pub fn add_bundle_to_circuit(&mut self, wires: &BinaryBundle<WireMod2>) {
        for wire in wires.wires() {
            self.add_wire_to_circuit(wire);
        }
    }

    /// Encode a wire, producing the zero wire as well as the encoded value.
    pub fn encode_wire(&mut self, val: u16) -> (WireMod2, WireMod2) {
        GCUtils::encode_wire(&mut self.rng, &self.delta, val)
    }

    /// Garbles an 'and' gate given two input wires and the delta.
    ///
    /// Outputs a tuple consisting of the two gates (that should be transfered to the evaluator)
    /// and the next wire label for the garbler.
    ///
    /// Used internally as a subroutine to implement 'and' gates for `FancyBinary`.
    fn garble_and_gate(&mut self, a: &WireMod2, b: &WireMod2) -> (Block, Block, WireMod2) {
        let gate_num = self.current_gate();
        GCUtils::garble_and_gate(gate_num, a, b, &self.delta)
    }
}

impl<N: Rep3Network> Fancy for Rep3Garbler<'_, N> {
    type Item = WireMod2;
    type Error = GarblerError;

    fn constant(&mut self, x: u16, q: u16) -> Result<WireMod2, GarblerError> {
        let zero = WireMod2::rand(&mut self.rng, q);
        let wire = zero.plus(self.delta.cmul_eq(x));
        self.add_wire_to_circuit(&wire);
        Ok(zero)
    }

    fn output(&mut self, x: &WireMod2) -> Result<Option<u16>, GarblerError> {
        let i = self.current_output();
        let d = self.delta;
        for k in 0..2 {
            let block = x.plus(&d.cmul(k)).hash(output_tweak(i, k));
            self.add_block_to_circuit(&block);
        }
        Ok(None)
    }
}

impl<N: Rep3Network> FancyBinary for Rep3Garbler<'_, N> {
    fn and(&mut self, a: &Self::Item, b: &Self::Item) -> Result<Self::Item, Self::Error> {
        let (gate0, gate1, c) = self.garble_and_gate(a, b);
        self.add_block_to_circuit(&gate0);
        self.add_block_to_circuit(&gate1);
        Ok(c)
    }

    fn xor(&mut self, x: &Self::Item, y: &Self::Item) -> Result<Self::Item, Self::Error> {
        Ok(x.plus(y))
    }

    /// We can negate by having garbler xor wire with Delta
    ///
    /// Since we treat all garbler wires as zero,
    /// xoring with delta conceptually negates the value of the wire
    fn negate(&mut self, x: &Self::Item) -> Result<Self::Item, Self::Error> {
        let delta = self.delta;
        self.xor(&delta, x)
    }
}
