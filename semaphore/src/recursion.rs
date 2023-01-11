use plonky2::iop::witness::{PartialWitness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::{CircuitConfig, VerifierCircuitData, VerifierCircuitTarget};
use plonky2::plonk::proof::ProofWithPublicInputs;

use crate::access_set::AccessSet;
use crate::signal::{Digest, PlonkyProof, Signal, C, F};

impl AccessSet {
    pub fn aggregate_signals(
        &self,
        topic0: Digest,
        signal0: Signal,
        topic1: Digest,
        signal1: Signal,
        verifier_data: &VerifierCircuitData<F, C, 2>,
    ) -> (Digest, Digest, PlonkyProof) {
        let config = CircuitConfig::standard_recursion_zk_config();
        let mut builder = CircuitBuilder::new(config);
        let mut partial_witness = PartialWitness::new();

        let public_inputs0: Vec<F> = self
            .0
            .cap
            .0
            .iter()
            .flat_map(|h| h.elements)
            .chain(signal0.nullifier)
            .chain(topic0)
            .collect();
        let public_inputs1: Vec<F> = self
            .0
            .cap
            .0
            .iter()
            .flat_map(|h| h.elements)
            .chain(signal1.nullifier)
            .chain(topic1)
            .collect();
        
        let proof_target0 = builder.add_virtual_proof_with_pis::<C>(&verifier_data.common);
        partial_witness.set_proof_with_pis_target(
            &proof_target0,
            &ProofWithPublicInputs {
                proof: signal0.proof,
                public_inputs: public_inputs0,
            },
        );

        let proof_target1 = builder.add_virtual_proof_with_pis::<C>(&verifier_data.common);
        partial_witness.set_proof_with_pis_target(
            &proof_target1,
            &ProofWithPublicInputs {
                proof: signal1.proof,
                public_inputs: public_inputs1,
            },
        );

        let verifier_data_target = VerifierCircuitTarget {
            constants_sigmas_cap: builder
                .add_virtual_cap(verifier_data.common.config.fri_config.cap_height),
            circuit_digest: builder.add_virtual_hash(),
        };

        partial_witness.set_cap_target(
            &verifier_data_target.constants_sigmas_cap,
            &verifier_data.verifier_only.constants_sigmas_cap,
        );

        builder.verify_proof::<C>(&proof_target0, &verifier_data_target, &verifier_data.common);
        builder.verify_proof::<C>(&proof_target1, &verifier_data_target, &verifier_data.common);

        let data = builder.build();
        let recursive_proof = data.prove(partial_witness).unwrap();

        data.verify(recursive_proof.clone()).unwrap();

        (signal0.nullifier, signal1.nullifier, recursive_proof.proof)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::{Result, Ok};
    use plonky2::field::types::{Field, Sample};
    use plonky2::hash::merkle_tree::MerkleTree;
    use plonky2::hash::poseidon::PoseidonHash;
    use plonky2::plonk::config::Hasher;

    use crate::access_set::AccessSet;
    use crate::signal::{Digest, F};

    #[test]
    fn test_recursion() -> Result<()> {
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
        let j = 3005;
        let topic0 = [F::rand(); 4];
        let topic1 = [F::rand(); 4];

        let time_signal = std::time::Instant::now();
        let (signal0, verifier_circuit_data0) =
            access_set.make_signal(private_keys[i], topic0, i)?;
        println!("done proving 1, elapsed: {:.2?}", time_signal.elapsed());

        let (signal1, verifier_circuit_data1) =
            access_set.make_signal(private_keys[j], topic1, j)?;
        println!("done proving 2, elapsed: {:.2?}", time_signal.elapsed());

        let recursion_now = std::time::Instant::now();
        let (nullifier0, nullifier1, recursive_proof) =
            access_set.aggregate_signals(topic0, signal0, 
            topic1, signal1, &verifier_circuit_data0);
        println!("done proving recursion, elapsed: {:.2?}", recursion_now.elapsed());
        Ok({})
    
    }

}