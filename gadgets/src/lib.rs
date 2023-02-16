use core::ops::Range;
use std::f32::consts::E;

use plonky2::gates::{multiplication_extension::MulExtensionGate, util::StridedConstraintConsumer};
use plonky2::iop::{
    ext_target::ExtensionTarget,
    generator::{GeneratedValues, SimpleGenerator, WitnessGenerator},
    target::Target,
    witness::{PartitionWitness, Witness, WitnessWrite},
};
use plonky2::plonk::{
    circuit_builder::CircuitBuilder,
    circuit_data::CircuitConfig,
    vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase},
};
use plonky2::{
    field::extension::{Extendable, FieldExtension},
    gates::gate::Gate,
    hash::hash_types::RichField,
};

#[derive(Debug)]
pub(crate) struct NumericCustomGate<const D: usize> {
    // Number of operations performed by the gate
    num_ops: usize,
}

impl<const D: usize> NumericCustomGate<D> {
    pub fn new_from_config(config: &CircuitConfig) -> Self {
        Self {
            num_ops: Self::num_ops(config),
        }
    }

    pub(crate) fn num_ops(config: &CircuitConfig) -> usize {
        let wires_per_op = 3 * D;
        config.num_routed_wires / wires_per_op
    }

    pub fn wires_multiplicand_0(i: usize) -> Range<usize> {
        3 * D * i..3 * D * i + D
    }

    pub fn wires_multiplicand_1(i: usize) -> Range<usize> {
        3 * D * i + D..3 * D * i + 2 * D
    }

    pub fn wires_output(i: usize) -> Range<usize> {
        3 * D * i + 2 * D..3 * D * i + 3 * D
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for NumericCustomGate<D> {
    fn id(&self) -> String {
        format!("{self:?}<D={D}>")
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<<F as Extendable<D>>::Extension> {
        let local_constants = vars.local_constants;
        let local_wires = vars.local_wires;

        let mut constraints = vec![];
        for i in 0..self.num_ops {
            let multiplicand_0 = vars.get_local_ext_algebra(Self::wires_multiplicand_0(i));
            let multiplicand_1 = vars.get_local_ext_algebra(Self::wires_multiplicand_1(i));
            let output = vars.get_local_ext_algebra(Self::wires_output(i));
            // fields have (+, *) which are both associative, commutative and the distribution law holds a * (b + c) = a * b + a * c
            let computed_output =
                (multiplicand_0 * multiplicand_1) * (multiplicand_0 * multiplicand_1); // (a * b)^2 == (a * b) * (a * b) == a * (b * (a * b)) == a * ((b * a) * b) == a * ((a * b) * b)) == (a * a) * (b * b) == (a^2) * (b^2)

            constraints.extend((output - computed_output).to_basefield_array());
        }

        constraints
    }

    fn eval_unfiltered_base_one(
        &self,
        vars: EvaluationVarsBase<F>,
        mut yield_constr: StridedConstraintConsumer<F>,
    ) {
        let local_constants = vars.local_constants;
        let local_wires = vars.local_wires;

        for i in 0..self.num_ops {
            let multiplicand_0 = vars.get_local_ext(Self::wires_multiplicand_0(i));
            let multiplicand_1 = vars.get_local_ext(Self::wires_multiplicand_1(i));
            let output = vars.get_local_ext(Self::wires_output(i));
            let computed_output =
                (multiplicand_0 * multiplicand_1) * (multiplicand_0 * multiplicand_1);

            yield_constr.many((output - computed_output).to_basefield_array());
        }
    }

    fn eval_unfiltered_circuit(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let local_constants = vars.local_constants;
        let local_wires = vars.local_wires;

        let mut constraints = vec![];
        for i in 0..self.num_ops {
            let multiplicand_0 = vars.get_local_ext_algebra(Self::wires_multiplicand_0(i));
            let multiplicand_1 = vars.get_local_ext_algebra(Self::wires_multiplicand_1(i));
            let output = vars.get_local_ext_algebra(Self::wires_output(i));
            let intermediate_mult = builder.mul_ext_algebra(multiplicand_0, multiplicand_1);
            let computed_output = builder.mul_ext_algebra(intermediate_mult, intermediate_mult);

            let diff = builder.sub_ext_algebra(output, computed_output);
            constraints.extend(diff.to_ext_target_array());
        }

        constraints
    }

    fn generators(
        &self,
        row: usize,
        local_constants: &[F],
    ) -> Vec<Box<dyn plonky2::iop::generator::WitnessGenerator<F>>> {
        (0..<NumericCustomGate<D> as Gate<F, D>>::num_ops(&self))
            .map(|i| {
                let g: Box<dyn WitnessGenerator<F>> = Box::new(
                    NumericCustomGenerator {
                        row,
                        const_0: F::ONE,
                        i,
                    }
                    .adapter(),
                );
                g
            })
            .collect()
    }

    fn degree(&self) -> usize {
        4
    }

    fn num_constants(&self) -> usize {
        1
    }

    fn num_wires(&self) -> usize {
        <NumericCustomGate<D> as Gate<F, D>>::num_ops(&self) * 4
    }

    fn num_constraints(&self) -> usize {
        <NumericCustomGate<D> as Gate<F, D>>::num_ops(&self) * D
    }
}

#[derive(Clone, Debug)]
struct NumericCustomGenerator<F: RichField + Extendable<D>, const D: usize> {
    row: usize,
    const_0: F,
    i: usize,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for NumericCustomGenerator<F, D>
{
    fn dependencies(&self) -> Vec<plonky2::iop::target::Target> {
        NumericCustomGate::<D>::wires_multiplicand_0(self.i)
            .chain(NumericCustomGate::<D>::wires_multiplicand_1(self.i))
            .map(|i| Target::wire(self.row, i))
            .collect()
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let extract_extension = |range: Range<usize>| -> F::Extension {
            let t = ExtensionTarget::from_range(self.row, range);
            witness.get_extension_target(t)
        };

        let multiplicand_0 =
            extract_extension(NumericCustomGate::<D>::wires_multiplicand_0(self.i));
        let multiplicand_1 =
            extract_extension(NumericCustomGate::<D>::wires_multiplicand_1(self.i));

        let output_target =
            ExtensionTarget::from_range(self.row, NumericCustomGate::<D>::wires_output(self.i));
        let computed_output = (multiplicand_0 * multiplicand_1) * (multiplicand_0 * multiplicand_1);

        out_buffer.set_extension_target(output_target, computed_output)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;
    use crate::field::goldilocks_field::GoldilocksField;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    #[test]
    fn low_degree() {
        let gate = NumericCustomGate::new_from_config(&CircuitConfig::standard_recursion_config());
        test_low_degree::<GoldilocksField, _, 4>(gate);
    }

    #[test]
    fn eval_fns() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let gate = NumericCustomGate::new_from_config(&CircuitConfig::standard_recursion_config());
        test_eval_fns::<F, C, _, D>(gate)
    }
}
