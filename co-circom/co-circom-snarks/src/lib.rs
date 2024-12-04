#![warn(missing_docs)]
//! This crate collects all functionality that is shared between the SNARKs supported by co-circom. At the moment
//! this is [Groth16](https://eprint.iacr.org/2016/260.pdf) and [PLONK](https://eprint.iacr.org/2019/953.pdf).

use ark_ff::PrimeField;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use circom_types::Witness;
use mpc_core::protocols::{
    rep3::{self, MaybeRep3ShareVecType, Rep3PrimeFieldShare, Rep3ShareVecType},
    shamir::{self, ShamirPrimeFieldShare},
};
use rand::{distributions::Standard, prelude::Distribution, CryptoRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;

/// This type represents the serialized version of a Rep3 witness. Its share can be either additive or replicated, and in both cases also compressed.
#[derive(Debug, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct SerializeableSharedRep3Witness<F: PrimeField, U: Rng + SeedableRng + CryptoRng>
where
    U::Seed: Serialize + for<'a> Deserialize<'a> + Clone + std::fmt::Debug,
{
    /// The public inputs (which are the outputs of the circom circuit).
    /// This also includes the constant 1 at position 0.
    #[serde(
        serialize_with = "mpc_core::ark_se",
        deserialize_with = "mpc_core::ark_de"
    )]
    pub public_inputs: Vec<F>,
    /// The secret-shared witness elements.
    pub witness: Rep3ShareVecType<F, U>,
}

impl<F: PrimeField, U: Rng + SeedableRng + CryptoRng> SerializeableSharedRep3Witness<F, U>
where
    U::Seed: Serialize + for<'a> Deserialize<'a> + Clone + std::fmt::Debug,
{
    /// Transforms a shared witness into a serializable version.
    pub fn from_shared_witness(inp: SharedWitness<F, Rep3PrimeFieldShare<F>>) -> Self {
        Self {
            public_inputs: inp.public_inputs,
            witness: Rep3ShareVecType::Replicated(inp.witness),
        }
    }
}

//TODO THE SECRETSHARED TRAIT IS REALLY BAD. WE DO WANT SOMETHING ELSE!
/// A shared witness in the circom ecosystem.
#[derive(Debug, Serialize, Deserialize)]
pub struct SharedWitness<F: PrimeField, S>
where
    S: CanonicalSerialize + CanonicalDeserialize + Clone,
{
    #[serde(
        serialize_with = "mpc_core::ark_se",
        deserialize_with = "mpc_core::ark_de"
    )]
    /// The public inputs (which are the outputs of the circom circuit).
    /// This also includes the constant 1 at position 0.
    pub public_inputs: Vec<F>,
    #[serde(
        serialize_with = "mpc_core::ark_se",
        deserialize_with = "mpc_core::ark_de"
    )]
    /// The secret-shared witness elements.
    pub witness: Vec<S>,
}

/// This type represents the serialized version of a Rep3 witness. Its share can be either additive or replicated, and in both cases also compressed.
#[derive(Debug, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct SerializeableSharedRep3Input<F: PrimeField, U: Rng + SeedableRng + CryptoRng>
where
    U::Seed: Serialize + for<'a> Deserialize<'a> + Clone + std::fmt::Debug,
{
    #[serde(
        serialize_with = "mpc_core::ark_se",
        deserialize_with = "mpc_core::ark_de"
    )]
    /// A map from variable names to the public field elements.
    /// This is a BTreeMap because it implements Canonical(De)Serialize.
    pub public_inputs: BTreeMap<String, Vec<F>>,
    #[serde(
        serialize_with = "mpc_core::ark_se",
        deserialize_with = "mpc_core::ark_de"
    )]
    pub maybe_public_inputs: BTreeMap<String, Vec<Option<F>>>,
    /// A map from variable names to the share of the field element.
    /// This is a BTreeMap because it implements Canonical(De)Serialize.
    pub shared_inputs: BTreeMap<String, Rep3ShareVecType<F, U>>,
    /// A map from variable names to vecs with maybe unknown elements that need to be merged.
    /// This is a BTreeMap because it implements Canonical(De)Serialize.
    #[serde(default)]
    pub maybe_shared_inputs: BTreeMap<String, MaybeRep3ShareVecType<F>>,
}

impl<F: PrimeField, U: Rng + SeedableRng + CryptoRng> Default for SerializeableSharedRep3Input<F, U>
where
    U::Seed: Serialize + for<'a> Deserialize<'a> + Clone + std::fmt::Debug,
{
    fn default() -> Self {
        Self {
            public_inputs: BTreeMap::new(),
            maybe_public_inputs: BTreeMap::new(),
            shared_inputs: BTreeMap::new(),
            maybe_shared_inputs: BTreeMap::new(),
        }
    }
}

impl<F: PrimeField, U: Rng + SeedableRng + CryptoRng> SerializeableSharedRep3Input<F, U>
where
    U::Seed: Serialize + for<'a> Deserialize<'a> + Clone + std::fmt::Debug,
    Standard: Distribution<U::Seed>,
{
    /// Shares a given input into a [Rep3ShareVecType] type.
    pub fn share_rep3<R: Rng + CryptoRng>(
        input: &[F],
        rng: &mut R,
        seeded: bool,
        additive: bool,
    ) -> [Rep3ShareVecType<F, U>; 3] {
        match (seeded, additive) {
            (true, true) => {
                let [share1, share2, share3] =
                    rep3::share_field_elements_additive_seeded::<_, _, U>(input, rng);
                let share1 = Rep3ShareVecType::SeededAdditive(share1);
                let share2 = Rep3ShareVecType::SeededAdditive(share2);
                let share3 = Rep3ShareVecType::SeededAdditive(share3);
                [share1, share2, share3]
            }
            (true, false) => {
                let [share1, share2, share3] =
                    rep3::share_field_elements_seeded::<_, _, U>(input, rng);
                let share1 = Rep3ShareVecType::SeededReplicated(share1);
                let share2 = Rep3ShareVecType::SeededReplicated(share2);
                let share3 = Rep3ShareVecType::SeededReplicated(share3);
                [share1, share2, share3]
            }
            (false, true) => {
                let [share1, share2, share3] = rep3::share_field_elements_additive(input, rng);
                let share1 = Rep3ShareVecType::Additive(share1);
                let share2 = Rep3ShareVecType::Additive(share2);
                let share3 = Rep3ShareVecType::Additive(share3);
                [share1, share2, share3]
            }
            (false, false) => {
                let [share1, share2, share3] = rep3::share_field_elements(input, rng);
                let share1 = Rep3ShareVecType::Replicated(share1);
                let share2 = Rep3ShareVecType::Replicated(share2);
                let share3 = Rep3ShareVecType::Replicated(share3);
                [share1, share2, share3]
            }
        }
    }

    /// Shares a given input with unknown elements into a [MaybeRep3ShareVecType] type.
    pub fn maybe_share_rep3<R: Rng + CryptoRng>(
        input: &[Option<F>],
        rng: &mut R,
        additive: bool,
    ) -> [MaybeRep3ShareVecType<F>; 3] {
        if additive {
            let [share1, share2, share3] = rep3::share_maybe_field_elements_additive(input, rng);
            let share1 = MaybeRep3ShareVecType::Additive(share1);
            let share2 = MaybeRep3ShareVecType::Additive(share2);
            let share3 = MaybeRep3ShareVecType::Additive(share3);
            [share1, share2, share3]
        } else {
            let [share1, share2, share3] = rep3::share_maybe_field_elements(input, rng);
            let share1 = MaybeRep3ShareVecType::Replicated(share1);
            let share2 = MaybeRep3ShareVecType::Replicated(share2);
            let share3 = MaybeRep3ShareVecType::Replicated(share3);
            [share1, share2, share3]
        }
    }

    /// Merges two [SerializeableSharedRep3Input]s into one, performing basic sanity checks.
    pub fn merge(self, other: Self) -> eyre::Result<Self> {
        let mut shared_inputs = self.shared_inputs;
        let maybe_shared_inputs = self.maybe_shared_inputs;
        let public_inputs = self.public_inputs;
        let maybe_public_inputs = self.maybe_public_inputs;

        for (key, value) in other.public_inputs.iter() {
            if !public_inputs.contains_key(key) {
                eyre::bail!("Public input \"{key}\" must be present in all files");
            }
            if public_inputs.get(key).expect("is there we checked") != value {
                eyre::bail!("Public input \"{key}\" must be same in all files");
            }
        }

        let mut maybe_merged_public_inputs = BTreeMap::new();
        for key in other.maybe_public_inputs.keys() {
            if public_inputs.contains_key(key) {
                eyre::bail!("{key} is present in public inputs and maybe public inputs");
            }
            if !maybe_public_inputs.contains_key(key) {
                eyre::bail!("{key} is must be present in both maybe public inputs");
            }
            let mine = maybe_public_inputs.get(key).expect("we checked");
            let theirs = other.maybe_public_inputs.get(key).expect("we checked");
            let mut merged = Vec::with_capacity(maybe_public_inputs.len());
            for (mine, their) in mine.into_iter().zip(theirs.into_iter()) {
                match (mine, their) {
                    (Some(_), Some(_)) => eyre::bail!("Same index for maybe shared public inputs"),
                    (Some(mine), None) => merged.push(Some(mine.clone())),
                    (None, Some(their)) => merged.push(Some(their.clone())),
                    (None, None) => continue,
                }
            }
            maybe_merged_public_inputs.insert(key.to_string(), merged);
        }

        for (key, value) in other.shared_inputs {
            if shared_inputs.contains_key(&key) {
                eyre::bail!("Input with name {} present in multiple input shares", key);
            }
            if public_inputs.contains_key(&key) || other.public_inputs.contains_key(&key) {
                eyre::bail!(
                    "Input name is once in shared inputs and once in public inputs: \"{key}\""
                );
            }
            shared_inputs.insert(key, value);
        }

        let mut merged_maybe_shared_inputs = BTreeMap::new();
        if maybe_shared_inputs.len() != other.maybe_shared_inputs.len() {
            eyre::bail!("Both inputs must have the same number of unmerged entries");
        }
        for ((k1, v1), (k2, v2)) in maybe_shared_inputs
            .into_iter()
            .zip(other.maybe_shared_inputs.into_iter())
        {
            if k1 != k2 {
                eyre::bail!("Both inputs must have the same keys for unmerged elements");
            }

            match (v1, v2) {
                (
                    MaybeRep3ShareVecType::Replicated(shares),
                    MaybeRep3ShareVecType::Replicated(other_shares),
                ) => {
                    let merged = shares
                        .into_iter()
                        .zip(other_shares.into_iter())
                        .map(|(a, b)| match (a, b) {
                            (None, None) => Ok(None),
                            (a @ Some(_), None) | (None, a @ Some(_)) => Ok(a),
                            _ => Err(eyre::eyre!("Input {} present in both unmerged inputs", k1)),
                        })
                        .collect::<Result<Vec<_>, eyre::Report>>()?;
                    if let Some(merged) = merged.iter().cloned().collect::<Option<Vec<_>>>() {
                        shared_inputs.insert(k1.clone(), Rep3ShareVecType::Replicated(merged));
                    } else {
                        merged_maybe_shared_inputs
                            .insert(k1.clone(), MaybeRep3ShareVecType::Replicated(merged));
                    }
                }
                (
                    MaybeRep3ShareVecType::Additive(shares),
                    MaybeRep3ShareVecType::Additive(other_shares),
                ) => {
                    let merged = shares
                        .into_iter()
                        .zip(other_shares.into_iter())
                        .map(|(a, b)| match (a, b) {
                            (None, None) => Ok(None),
                            (a @ Some(_), None) | (None, a @ Some(_)) => Ok(a),
                            _ => Err(eyre::eyre!("Input {} present in both unmerged inputs", k1)),
                        })
                        .collect::<Result<Vec<_>, eyre::Report>>()?;
                    if let Some(merged) = merged.iter().cloned().collect::<Option<Vec<_>>>() {
                        shared_inputs.insert(k1.clone(), Rep3ShareVecType::Additive(merged));
                    } else {
                        merged_maybe_shared_inputs
                            .insert(k1.clone(), MaybeRep3ShareVecType::Additive(merged));
                    }
                }
                _ => {
                    eyre::bail!("Input {} cannot be merged, the share type is different", k1);
                }
            }
        }

        Ok(Self {
            public_inputs,
            maybe_public_inputs: maybe_merged_public_inputs,
            shared_inputs,
            maybe_shared_inputs: merged_maybe_shared_inputs,
        })
    }
}

/// A shared input for a collaborative circom witness extension.
#[derive(Debug, Serialize, Deserialize)]
pub struct SharedInput<F: PrimeField, S>
where
    S: CanonicalSerialize + CanonicalDeserialize + Clone,
{
    #[serde(
        serialize_with = "mpc_core::ark_se",
        deserialize_with = "mpc_core::ark_de"
    )]
    /// A map from variable names to the public field elements.
    /// This is a BTreeMap because it implements Canonical(De)Serialize.
    pub public_inputs: BTreeMap<String, Vec<F>>,
    #[serde(
        serialize_with = "mpc_core::ark_se",
        deserialize_with = "mpc_core::ark_de"
    )]
    /// A map from variable names to the share of the field element.
    /// This is a BTreeMap because it implements Canonical(De)Serialize.
    pub shared_inputs: BTreeMap<String, Vec<S>>,
}

/// We manually implement Clone here since it was not derived correctly and it added bounds on T, P which are not needed
impl<F: PrimeField, S> Clone for SharedWitness<F, S>
where
    S: CanonicalSerialize + CanonicalDeserialize + Clone,
{
    fn clone(&self) -> Self {
        Self {
            public_inputs: self.public_inputs.clone(),
            witness: self.witness.clone(),
        }
    }
}

/// We manually implement Clone here since it was not derived correctly and it added bounds on T, P which are not needed
impl<F: PrimeField, S> Clone for SharedInput<F, S>
where
    S: CanonicalSerialize + CanonicalDeserialize + Clone,
{
    fn clone(&self) -> Self {
        Self {
            public_inputs: self.public_inputs.clone(),
            shared_inputs: self.shared_inputs.clone(),
        }
    }
}

impl<F: PrimeField, S> Default for SharedInput<F, S>
where
    S: CanonicalSerialize + CanonicalDeserialize + Clone,
{
    fn default() -> Self {
        Self {
            public_inputs: BTreeMap::new(),
            shared_inputs: BTreeMap::new(),
        }
    }
}

impl<F: PrimeField> SharedInput<F, Rep3PrimeFieldShare<F>> {
    pub fn build_from_sources<U>(
        sources: Vec<SerializeableSharedRep3Input<F, U>>,
    ) -> eyre::Result<Self>
    where
        U: Rng + SeedableRng + CryptoRng,
        U::Seed: Serialize + for<'a> Deserialize<'a> + Clone + std::fmt::Debug,
    {
        let mut shared_input = Self::default();
        let mut maybe_publics: BTreeMap<String, Vec<Option<F>>> = BTreeMap::new();
        let mut maybe_shared = BTreeMap::new();
        for source in sources {
            for (k, v) in source.public_inputs {
                if let Some(old) = shared_input.public_inputs.insert(k, v.clone()) {
                    if old != v {
                        eyre::bail!("public inputs must match from sources");
                    }
                }
            }

            for (k, v) in source.shared_inputs {
                match v {
                    Rep3ShareVecType::Replicated(rep3) => {
                        if shared_input.shared_inputs.insert(k, rep3).is_some() {
                            eyre::bail!("cannot provide multiple shared inputs with same key")
                        }
                    }
                    Rep3ShareVecType::SeededReplicated(_) => todo!(),
                    Rep3ShareVecType::Additive(_) => todo!(),
                    Rep3ShareVecType::SeededAdditive(_) => todo!(),
                }
            }

            for (k, v) in source.maybe_public_inputs {
                // is there already a maybe public?
                if let Some(mine) = maybe_publics.remove(&k) {
                    let mut merged = Vec::with_capacity(mine.len());
                    if mine.len() != v.len() {
                        eyre::bail!("maybe public inputs must be same length");
                    }
                    for (mine, their) in mine.into_iter().zip(v) {
                        match (mine, their) {
                            (Some(m), Some(t)) => {
                                if m != t {
                                    eyre::bail!("maybe public inputs must be same!");
                                }
                                merged.push(Some(m));
                            }
                            (None, Some(f)) | (Some(f), None) => merged.push(Some(f)),
                            (None, None) => merged.push(None),
                        }
                    }
                    maybe_publics.insert(k, merged);
                } else {
                    // does not exist, just add it
                    maybe_publics.insert(k, v);
                }
            }

            for (k, theirs) in source.maybe_shared_inputs {
                // is there already a maybe shared?
                if let Some(mine) = maybe_shared.remove(&k) {
                    let mut merged = vec![];
                    match (mine, theirs) {
                        (
                            MaybeRep3ShareVecType::Replicated(mine),
                            MaybeRep3ShareVecType::Replicated(theirs),
                        ) => {
                            for (mine, their) in mine.into_iter().zip(theirs) {
                                match (mine, their) {
                                    (Some(m), Some(t)) => {
                                        if m != t {
                                            eyre::bail!("maybe public inputs must be same!");
                                        }
                                        merged.push(Some(m));
                                    }
                                    (None, Some(f)) | (Some(f), None) => merged.push(Some(f)),
                                    (None, None) => merged.push(None),
                                }
                            }
                        }
                        (
                            MaybeRep3ShareVecType::Replicated(_),
                            MaybeRep3ShareVecType::Additive(_),
                        ) => todo!(),
                        (
                            MaybeRep3ShareVecType::Additive(_),
                            MaybeRep3ShareVecType::Replicated(_),
                        ) => todo!(),
                        (
                            MaybeRep3ShareVecType::Additive(_),
                            MaybeRep3ShareVecType::Additive(_),
                        ) => todo!(),
                    }
                    maybe_shared.insert(k, MaybeRep3ShareVecType::Replicated(merged));
                } else {
                    // does not exist, just add it
                    maybe_shared.insert(k, theirs);
                }
            }
        }
        // now check if all is present
        for (k, v) in maybe_publics {
            if shared_input.public_inputs.contains_key(&k) {
                eyre::bail!("key present \"{k}\"in maybe shared and in public input");
            }
            let not_maybe = v
                .into_iter()
                .map(|v| v.ok_or(eyre::eyre!("Still unmerged public input")))
                .collect::<eyre::Result<Vec<_>>>()?;
            shared_input.public_inputs.insert(k, not_maybe);
        }

        for (k, v) in maybe_shared {
            if shared_input.shared_inputs.contains_key(&k) {
                eyre::bail!("key present \"{k}\"in maybe shared and in shared input");
            }
            match v {
                MaybeRep3ShareVecType::Replicated(rep3) => {
                    let not_maybe = rep3
                        .into_iter()
                        .map(|v| v.ok_or(eyre::eyre!("Still unmerged public input")))
                        .collect::<eyre::Result<Vec<_>>>()?;
                    shared_input.shared_inputs.insert(k, not_maybe);
                }
                MaybeRep3ShareVecType::Additive(_) => todo!(),
            }
        }
        Ok(shared_input)
    }
}

impl<F: PrimeField, S> SharedInput<F, S>
where
    S: CanonicalSerialize + CanonicalDeserialize + Clone,
{
    /// Adds a public input with a given name to the [SharedInput].
    pub fn add_public_input(&mut self, key: String, elements: Vec<F>) {
        self.public_inputs.insert(key, elements);
    }

    /// Adds a shared input with a given name to the [SharedInput].
    pub fn add_shared_input(&mut self, key: String, elements: Vec<S>) {
        self.shared_inputs.insert(key, elements);
    }

    /// Merges two [SharedInput]s into one, performing basic sanity checks.
    pub fn merge(self, other: Self) -> eyre::Result<Self> {
        let mut shared_inputs = self.shared_inputs;
        let public_inputs = self.public_inputs;
        for (key, value) in other.shared_inputs {
            if shared_inputs.contains_key(&key) {
                eyre::bail!("Input with name {} present in multiple input shares", key);
            }
            if public_inputs.contains_key(&key) || other.public_inputs.contains_key(&key) {
                eyre::bail!(
                    "Input name is once in shared inputs and once in public inputs: \"{key}\""
                );
            }
            shared_inputs.insert(key, value);
        }
        for (key, value) in other.public_inputs {
            if !public_inputs.contains_key(&key) {
                eyre::bail!("Public input \"{key}\" must be present in all files");
            }
            if public_inputs.get(&key).expect("is there we checked") != &value {
                eyre::bail!("Public input \"{key}\" must be same in all files");
            }
        }

        Ok(Self {
            shared_inputs,
            public_inputs,
        })
    }
}

impl<F: PrimeField, U: Rng + SeedableRng + CryptoRng> SerializeableSharedRep3Witness<F, U>
where
    U::Seed: Serialize + for<'a> Deserialize<'a> + Clone + std::fmt::Debug,

    Standard: Distribution<U::Seed>,
{
    /// Shares a given witness and public input vector using the Rep3 protocol.
    pub fn share_rep3<R: Rng + CryptoRng>(
        witness: Witness<F>,
        num_pub_inputs: usize,
        rng: &mut R,
        seeded: bool,
        additive: bool,
    ) -> [Self; 3] {
        let public_inputs = &witness.values[..num_pub_inputs];
        let witness = &witness.values[num_pub_inputs..];

        let [share1, share2, share3] =
            SerializeableSharedRep3Input::share_rep3(witness, rng, seeded, additive);

        let witness1 = Self {
            public_inputs: public_inputs.to_vec(),
            witness: share1,
        };
        let witness2 = Self {
            public_inputs: public_inputs.to_vec(),
            witness: share2,
        };
        let witness3 = Self {
            public_inputs: public_inputs.to_vec(),
            witness: share3,
        };
        [witness1, witness2, witness3]
    }
}

impl<F: PrimeField> SharedWitness<F, Rep3PrimeFieldShare<F>> {
    /// Shares a given witness and public input vector using the rep3 protocol.
    pub fn share_rep3<R: Rng + CryptoRng>(
        witness: Witness<F>,
        num_pub_inputs: usize,
        rng: &mut R,
    ) -> [Self; 3] {
        let public_inputs = &witness.values[..num_pub_inputs];
        let witness = &witness.values[num_pub_inputs..];
        let [share1, share2, share3] = rep3::share_field_elements(witness, rng);
        let witness1 = Self {
            public_inputs: public_inputs.to_vec(),
            witness: share1,
        };
        let witness2 = Self {
            public_inputs: public_inputs.to_vec(),
            witness: share2,
        };
        let witness3 = Self {
            public_inputs: public_inputs.to_vec(),
            witness: share3,
        };
        [witness1, witness2, witness3]
    }
}

impl<F: PrimeField> SharedWitness<F, ShamirPrimeFieldShare<F>> {
    /// Shares a given witness and public input vector using the Shamir protocol.
    pub fn share_shamir<R: Rng + CryptoRng>(
        witness: Witness<F>,
        num_pub_inputs: usize,
        degree: usize,
        num_parties: usize,
        rng: &mut R,
    ) -> Vec<Self> {
        let public_inputs = &witness.values[..num_pub_inputs];
        let witness = &witness.values[num_pub_inputs..];
        let shares = shamir::share_field_elements(witness, degree, num_parties, rng);
        shares
            .into_iter()
            .map(|share| Self {
                public_inputs: public_inputs.to_vec(),
                witness: share,
            })
            .collect()
    }
}

/// The error type for the verification of a Circom proof.
///
/// If the verification failed because the proof is Invalid, the method
/// will return the [VerificationError::InvalidProof] variant. If the
/// underlying implementation encounters an error, the method
/// will wrap that error in the [VerificationError::Malformed] variant.
#[derive(Debug)]
pub enum VerificationError {
    /// Indicates that the proof verification failed
    InvalidProof,
    /// Wraps an underlying error (e.g., malformed verification key)
    Malformed(eyre::Report),
}

impl From<eyre::Report> for VerificationError {
    fn from(error: eyre::Report) -> Self {
        VerificationError::Malformed(error)
    }
}

impl std::error::Error for VerificationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            VerificationError::Malformed(source) => Some(source.as_ref()),
            VerificationError::InvalidProof => None,
        }
    }
}

impl std::fmt::Display for VerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerificationError::InvalidProof => writeln!(f, "proof is invalid"),
            VerificationError::Malformed(error) => writeln!(f, "cannot verify proof: {error}"),
        }
    }
}

/// Gathers utility methods for proving coSNARKs.
pub mod utils {
    use ark_ff::{FftField, LegendreSymbol, PrimeField};
    use num_traits::ToPrimitive;

    /// Computes the roots of unity over the provided prime field. This method
    /// is equivalent with [circom's implementation](https://github.com/iden3/ffjavascript/blob/337b881579107ab74d5b2094dbe1910e33da4484/src/wasm_field1.js).
    ///
    /// We calculate smallest quadratic non residue q (by checking q^((p-1)/2)=-1 mod p). We also calculate smallest t s.t. p-1=2^s*t, s is the two adicity.
    /// We use g=q^t (this is a 2^s-th root of unity) as (some kind of) generator and compute another domain by repeatedly squaring g, should get to 1 in the s+1-th step.
    /// Then if log2(\text{domain_size}) equals s we take q^2 as root of unity. Else we take the log2(\text{domain_size}) + 1-th element of the domain created above.
    pub fn roots_of_unity<F: PrimeField + FftField>() -> (F, Vec<F>) {
        let mut roots = vec![F::zero(); F::TWO_ADICITY.to_usize().unwrap() + 1];
        let mut q = F::one();
        while q.legendre() != LegendreSymbol::QuadraticNonResidue {
            q += F::one();
        }
        let z = q.pow(F::TRACE);
        roots[0] = z;
        for i in 1..roots.len() {
            roots[i] = roots[i - 1].square();
        }
        roots.reverse();
        (q, roots)
    }
}
