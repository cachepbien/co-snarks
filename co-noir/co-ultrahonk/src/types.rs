use co_builder::prelude::{Polynomial, PrecomputedEntities};
use serde::{Deserialize, Serialize};
use ultrahonk::prelude::{ShiftedTableEntities, ShiftedWitnessEntities};

// This is what we get from the proving key, we shift at a later point
#[derive(Default, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct Polynomials<Shared: Default, Public: Default>
where
    Polynomial<Shared>: Serialize + for<'a> Deserialize<'a>,
    Polynomial<Public>: Serialize + for<'a> Deserialize<'a>,
{
    pub witness: ProverWitnessEntities<Polynomial<Shared>, Polynomial<Public>>,
    pub precomputed: PrecomputedEntities<Polynomial<Public>>,
}

impl<Shared: Clone + Default, Public: Clone + Default> Polynomials<Shared, Public>
where
    Polynomial<Shared>: Serialize + for<'a> Deserialize<'a>,
    Polynomial<Public>: Serialize + for<'a> Deserialize<'a>,
{
    pub(crate) fn new(circuit_size: usize) -> Self {
        let mut polynomials = Self::default();
        // Shifting is done at a later point
        polynomials
            .witness
            .get_wires_mut()
            .iter_mut()
            .for_each(|el| el.resize(circuit_size, Default::default()));
        polynomials
            .witness
            .lookup_read_counts_and_tags_mut()
            .iter_mut()
            .for_each(|el| el.resize(circuit_size, Default::default()));
        polynomials.precomputed.iter_mut().for_each(|el| {
            el.resize(circuit_size, Default::default());
        });

        polynomials
    }
}

#[derive(Default)]
pub(crate) struct AllEntities<Shared: Default, Public: Default> {
    pub(crate) witness: WitnessEntities<Shared, Public>,
    pub(crate) precomputed: PrecomputedEntities<Public>,
    pub(crate) shifted_witness: ShiftedWitnessEntities<Shared>,
    pub(crate) shifted_tables: ShiftedTableEntities<Public>,
}

impl<Shared: Default, Public: Default> AllEntities<Shared, Public> {
    pub(crate) fn public_iter(&self) -> impl Iterator<Item = &Public> {
        self.precomputed
            .iter()
            .chain(self.witness.public_iter())
            .chain(self.shifted_tables.iter())
    }

    pub(crate) fn shared_iter(&self) -> impl Iterator<Item = &Shared> {
        self.witness
            .shared_iter()
            .chain(self.shifted_witness.iter())
    }

    pub(crate) fn into_shared_iter(self) -> impl Iterator<Item = Shared> {
        self.witness.into_shared_iter().chain(self.shifted_witness)
    }

    pub(crate) fn public_iter_mut(&mut self) -> impl Iterator<Item = &mut Public> {
        self.precomputed
            .iter_mut()
            .chain(self.witness.public_iter_mut())
            .chain(self.shifted_tables.iter_mut())
    }

    pub(crate) fn shared_iter_mut(&mut self) -> impl Iterator<Item = &mut Shared> {
        self.witness
            .shared_iter_mut()
            .chain(self.shifted_witness.iter_mut())
    }
}

impl<Shared: Default + Clone, Public: Default + Clone> AllEntities<Vec<Shared>, Vec<Public>> {
    pub(crate) fn new(circuit_size: usize) -> Self {
        let mut polynomials = Self::default();
        // Shifting is done at a later point
        polynomials
            .shared_iter_mut()
            .for_each(|el| el.resize(circuit_size, Default::default()));
        polynomials
            .public_iter_mut()
            .for_each(|el| el.resize(circuit_size, Default::default()));

        polynomials
    }
}

impl<T: Default> AllEntities<T, T> {
    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        self.precomputed
            .iter()
            .chain(self.witness.shared_iter())
            .chain(self.witness.public_iter())
            .chain(self.shifted_tables.iter())
            .chain(self.shifted_witness.iter())
    }
}

const PROVER_PRIVATE_WITNESS_ENTITIES_SIZE: usize = 4;
const PROVER_PUBLIC_WITNESS_ENTITIES_SIZE: usize = 2;
#[derive(Default, Serialize, Deserialize)]
pub struct ProverWitnessEntities<Shared, Public> {
    pub private_elements: [Shared; PROVER_PRIVATE_WITNESS_ENTITIES_SIZE],
    pub public_elements: [Public; PROVER_PUBLIC_WITNESS_ENTITIES_SIZE],
}

impl<Shared, Public> ProverWitnessEntities<Shared, Public> {
    const W_L: usize = 0; // column 0
    pub(crate) const W_R: usize = 1; // column 1
    const W_O: usize = 2; // column 2
    const W_4: usize = 3; // column 3 (modified by prover)

    const LOOKUP_READ_COUNTS: usize = 0; // column 6
    const LOOKUP_READ_TAGS: usize = 1; // column 7

    // const Z_PERM: usize = 4; // column 4 (computed by prover)
    // const LOOKUP_INVERSES: usize = 5; // column 5 (computed by prover);

    pub(crate) fn into_wires(self) -> impl Iterator<Item = Shared> {
        self.private_elements
            .into_iter()
            // .skip(Self::W_L)
            .take(Self::W_4 + 1 - Self::W_L)
    }

    pub(crate) fn get_wires_mut(&mut self) -> &mut [Shared] {
        &mut self.private_elements[Self::W_L..=Self::W_4]
    }

    pub(crate) fn w_l(&self) -> &Shared {
        &self.private_elements[Self::W_L]
    }

    pub(crate) fn w_r(&self) -> &Shared {
        &self.private_elements[Self::W_R]
    }

    pub(crate) fn w_o(&self) -> &Shared {
        &self.private_elements[Self::W_O]
    }

    pub(crate) fn w_4(&self) -> &Shared {
        &self.private_elements[Self::W_4]
    }

    pub(crate) fn lookup_read_counts(&self) -> &Public {
        &self.public_elements[Self::LOOKUP_READ_COUNTS]
    }

    pub(crate) fn lookup_read_tags(&self) -> &Public {
        &self.public_elements[Self::LOOKUP_READ_TAGS]
    }

    pub(crate) fn lookup_read_counts_and_tags_mut(&mut self) -> &mut [Public] {
        &mut self.public_elements[Self::LOOKUP_READ_COUNTS..Self::LOOKUP_READ_TAGS + 1]
    }
}

const PRIVATE_WITNESS_ENTITIES_SIZE: usize = 6;
const PUBLIC_WITNESS_ENTITIES_SIZE: usize = 2;
#[derive(Default)]
pub(crate) struct WitnessEntities<Shared, Public> {
    pub(crate) private_elements: [Shared; PRIVATE_WITNESS_ENTITIES_SIZE],
    pub(crate) public_elements: [Public; PUBLIC_WITNESS_ENTITIES_SIZE],
}

impl<Shared, Public> WitnessEntities<Shared, Public> {
    const W_L: usize = 0; // column 0
    const W_R: usize = 1; // column 1
    const W_O: usize = 2; // column 2
    const W_4: usize = 3; // column 3 (computed by prover)
    const Z_PERM: usize = 4; // column 4 (computed by prover)
    pub(crate) const LOOKUP_INVERSES: usize = 5; // column 5 (computed by prover);

    pub(crate) const LOOKUP_READ_COUNTS: usize = 0; // column 6
    pub(crate) const LOOKUP_READ_TAGS: usize = 1; // column 7

    pub(crate) fn shared_iter(&self) -> impl Iterator<Item = &Shared> {
        self.private_elements.iter()
    }

    pub(crate) fn public_iter(&self) -> impl Iterator<Item = &Public> {
        self.public_elements.iter()
    }

    pub(crate) fn into_shared_iter(self) -> impl Iterator<Item = Shared> {
        self.private_elements.into_iter()
    }

    pub(crate) fn shared_iter_mut(&mut self) -> impl Iterator<Item = &mut Shared> {
        self.private_elements.iter_mut()
    }

    pub(crate) fn public_iter_mut(&mut self) -> impl Iterator<Item = &mut Public> {
        self.public_elements.iter_mut()
    }

    pub(crate) fn to_be_shifted_mut(&mut self) -> &mut [Shared] {
        &mut self.private_elements[Self::W_L..=Self::Z_PERM]
    }

    pub(crate) fn w_l(&self) -> &Shared {
        &self.private_elements[Self::W_L]
    }

    pub(crate) fn w_r(&self) -> &Shared {
        &self.private_elements[Self::W_R]
    }

    pub(crate) fn w_o(&self) -> &Shared {
        &self.private_elements[Self::W_O]
    }

    pub(crate) fn w_4(&self) -> &Shared {
        &self.private_elements[Self::W_4]
    }

    pub(crate) fn z_perm(&self) -> &Shared {
        &self.private_elements[Self::Z_PERM]
    }

    pub(crate) fn lookup_inverses(&self) -> &Shared {
        &self.private_elements[Self::LOOKUP_INVERSES]
    }

    pub(crate) fn lookup_read_counts(&self) -> &Public {
        &self.public_elements[Self::LOOKUP_READ_COUNTS]
    }

    pub(crate) fn lookup_read_tags(&self) -> &Public {
        &self.public_elements[Self::LOOKUP_READ_TAGS]
    }

    pub(crate) fn lookup_inverses_mut(&mut self) -> &mut Shared {
        &mut self.private_elements[Self::LOOKUP_INVERSES]
    }

    pub(crate) fn lookup_read_counts_mut(&mut self) -> &mut Public {
        &mut self.public_elements[Self::LOOKUP_READ_COUNTS]
    }

    pub(crate) fn lookup_read_tags_mut(&mut self) -> &mut Public {
        &mut self.public_elements[Self::LOOKUP_READ_TAGS]
    }
}
