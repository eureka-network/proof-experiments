use anyhow::Result;
use plonky2::field::extension::Extendable;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::target::Target;
use plonky2::iop::witness::{PartialWitness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::{CircuitConfig, CircuitData};
use plonky2::plonk::config::GenericConfig;
use plonky2::plonk::proof::ProofWithPublicInputs;

pub trait NumericInstructionsCircuit<F: Extendable<D> + RichField, const D: usize> {
    fn add_target(&mut self, builder: &mut CircuitBuilder<F, D>);
    fn square_targets(&mut self, builder: &mut CircuitBuilder<F, D>);
    fn mul_targets(&mut self, builder: &mut CircuitBuilder<F, D>) -> Option<Target>;
    fn register_public_inputs(&mut self, builder: &mut CircuitBuilder<F, D>);
    fn set_partial_witnesses(&mut self, values: Vec<F>) -> Result<(), String>;
    fn register_output(&mut self, target: Target, builder: &mut CircuitBuilder<F, D>);
}

pub struct Circuit<F: Extendable<D> + RichField, const D: usize> {
    config: CircuitConfig,
    targets: Vec<Target>,
    partial_witness: PartialWitness<F>,
}

pub struct CircuitOutputs<F: Extendable<D> + RichField, C: GenericConfig<D, F = F>, const D: usize>
{
    circuit_data: CircuitData<F, C, D>,
    proof_with_pis: ProofWithPublicInputs<F, C, D>,
}

impl<F: Extendable<D> + RichField, const D: usize> Circuit<F, D> {
    pub fn new() -> Self {
        let config = CircuitConfig::standard_recursion_config();
        Self::new_with_config(config)
    }

    pub fn new_with_config(config: CircuitConfig) -> Self {
        Self {
            config,
            targets: Vec::new(),
            partial_witness: PartialWitness::new(),
        }
    }

    pub fn build_circuit<C: GenericConfig<D, F = F>>(
        &mut self,
        witnesses: Vec<F>,
    ) -> CircuitOutputs<F, C, D> {
        let mut builder = CircuitBuilder::<F, D>::new(self.config.clone());
        let num_targets = witnesses.len();
        // add num_targets
        for _ in 0..num_targets {
            self.add_target(&mut builder);
        }
        // square the targets
        self.square_targets(&mut builder);
        // multiply targets
        let mult_targets = self.mul_targets(&mut builder);
        // register public inputs
        self.register_public_inputs(&mut builder);
        if let Some(target) = mult_targets {
            self.register_output(target, &mut builder);
        }
        // set partial witnesses
        self.set_partial_witnesses(witnesses)
            .expect("Number of values should coincide with targets");
        // build the underlying circuit
        let data = builder.build::<C>();
        // get the proof
        let proof = data
            .prove(self.partial_witness.clone())
            .expect("Unexpected behavior");

        CircuitOutputs {
            circuit_data: data,
            proof_with_pis: proof,
        }
    }

    pub fn verify_proof<C: GenericConfig<D, F = F>>(
        &self,
        proof_with_pis: ProofWithPublicInputs<F, C, D>,
        data: CircuitData<F, C, D>,
    ) -> Result<()> {
        data.verify(proof_with_pis)
    }
}

impl<F: Extendable<D> + RichField, const D: usize> NumericInstructionsCircuit<F, D>
    for Circuit<F, D>
{
    fn add_target(&mut self, builder: &mut CircuitBuilder<F, D>) {
        self.targets.push(builder.add_virtual_target());
    }

    fn square_targets(&mut self, builder: &mut CircuitBuilder<F, D>) {
        if self.targets.is_empty() {
            return;
        }

        for target in &self.targets {
            builder.square(*target);
        }
    }

    fn mul_targets(&mut self, builder: &mut CircuitBuilder<F, D>) -> Option<Target> {
        if self.targets.len() <= 1 {
            return self.targets.first().copied();
        }

        let mut prev_target = self.targets[0];
        let mut temp: Target = Target::VirtualTarget { index: 0 };
        for cur_target in &self.targets[1..] {
            temp = builder.mul(prev_target, *cur_target);
            prev_target = temp;
        }

        Some(temp)
    }

    fn register_public_inputs(&mut self, builder: &mut CircuitBuilder<F, D>) {
        let _ = self
            .targets
            .iter()
            .map(|t| builder.register_public_input(*t))
            .collect::<Vec<_>>();
    }

    fn register_output(&mut self, target: Target, builder: &mut CircuitBuilder<F, D>) {
        builder.register_public_input(target);
    }

    fn set_partial_witnesses(&mut self, witnesses: Vec<F>) -> Result<(), String> {
        if witnesses.len() != self.targets.len() {
            return Err(format!(
                "The user should provide {} values, not {}",
                self.targets.len(),
                witnesses.len(),
            ));
        }
        let _ = self
            .targets
            .iter()
            .zip(witnesses)
            .map(|(target, value)| self.partial_witness.set_target(*target, value))
            .collect::<Vec<_>>();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::field::types::Field;
    use plonky2::plonk::config::PoseidonGoldilocksConfig;

    use super::*;
    type F = GoldilocksField;
    type C = PoseidonGoldilocksConfig;

    #[test]
    fn it_works_simple_example_build_circuit() {
        // start with a simple example a = 2, b = 2, in which case
        // (a^2) * (b^2) = 4 * 4 = 16
        let mut circuit = Circuit::<F, 2>::new();

        let witnesses: Vec<F> = vec![F::TWO, F::TWO];
        let CircuitOutputs {
            circuit_data,
            proof_with_pis,
        } = circuit.build_circuit::<C>(witnesses);

        // verify the proof
        assert!(circuit.verify_proof(proof_with_pis, circuit_data).is_ok());
    }

    #[test]
    fn it_works_involved_example_build_circuit() {
        // let a = 4, b = 2, c = 7, d = 5, in which case
        // (a^2) * (b^2) * (c^2) * (d^2) = 78_400
        let mut circuit = Circuit::<F, 2>::new();

        let witnesses: Vec<F> = vec![
            F::TWO + F::TWO,
            F::TWO,
            F::TWO + F::TWO + F::TWO + F::ONE,
            F::TWO + F::TWO + F::ONE,
        ];
        let CircuitOutputs {
            circuit_data,
            proof_with_pis,
        } = circuit.build_circuit::<C>(witnesses);

        // verify the proof
        assert!(circuit.verify_proof(proof_with_pis, circuit_data).is_ok());
    }
}
