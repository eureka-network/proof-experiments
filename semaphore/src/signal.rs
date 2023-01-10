use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::plonk::proof::Proof;

pub type F = GoldilocksField;
pub type Digest = [F; 4];
pub type C = PoseidonGoldilocksConfig;
pub type PlonkyProof = Proof<F, PoseidonGoldilocksConfig, 2>;

#[derive(Debug, Clone)]
pub struct Signal {
    pub nullifier: Digest,
    pub proof: PlonkyProof,
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use plonky2::field::types::{Field, Sample};
    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::hash::merkle_tree::MerkleTree;
    use plonky2::hash::poseidon::PoseidonHash;
    use plonky2::plonk::config::Hasher;

    use crate::access_set::AccessSet;
    use crate::signal::{Digest, F, C};

    #[test]
    fn test_semaphore() -> Result<()> {

        let n = 1 << 20;
        let private_keys: Vec<Digest> = (0..n).map(|_| [F::rand(); 4]).collect();
        let public_keys: Vec<Vec<F>> = private_keys
            .iter()
            .map(|&sk| {
                PoseidonHash::hash_no_pad(&[sk, [F::ZERO; 4]].concat())
                    .elements
                    .to_vec()
            }).collect();
        let access_set = AccessSet(MerkleTree::new(public_keys, 0));

        let i = 12;
        let topic = [F::rand(); 4];

        let now = std::time::Instant::now();
        let (signal, verifier_circuit_data) =
            access_set.make_signal(private_keys[i], topic, i)?;
        println!("done proving, elapsed: {:.2?}", now.elapsed());

        access_set.verify_signal(topic, signal, &verifier_circuit_data)
    }
}
