use std::marker::PhantomData;

use anyhow::Result;
use plonky2::field::types::{PrimeField, Sample};
use plonky2::hash::hash_types::RichField;
use plonky2::iop::generator::SimpleGenerator;
use plonky2::iop::target::Target;
use plonky2::iop::witness::{PartialWitness, Witness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::CircuitConfig;
use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
use plonky2_field::extension::Extendable;

/// A generator to compute the n-th root of a given value x, outside the circuit,
/// so it can be used as an additional public input
#[derive(Debug)]
pub struct NthRootGenerator<F: RichField + Extendable<D>, const D: usize, const N: u64> {
    x: Target, // x = x_pow_n^{1 / n}
    x_pow_n: Target,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize, const N: u64> NthRootGenerator<F, D, N> {
    fn get_pow() -> u64 {
        N
    }
}

impl<F: RichField + Extendable<D>, const D: usize, const N: u64> SimpleGenerator<F>
    for NthRootGenerator<F, D, N>
{
    fn dependencies(&self) -> Vec<Target> {
        vec![self.x_pow_n]
    }

    fn run_once(
        &self,
        witness: &plonky2::iop::witness::PartitionWitness<F>,
        out_buffer: &mut plonky2::iop::generator::GeneratedValues<F>,
    ) {
        let x_pow_n = witness.get_target(self.x_pow_n);
        let nth_pow = Self::get_pow();

        if nth_pow == 0_u64 {
            return;
        }

        let x = x_pow_n.kth_root_u64(nth_pow);

        println!("The {nth_pow}th-root: {x}");

        out_buffer.set_target(self.x, x);
    }
}

fn main() -> Result<()> {
    const D: usize = 2;
    const N: u64 = 3; // 3 | p - 1, where p = 2^64 - 2^32 + 1
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    let config = CircuitConfig::standard_recursion_config();
    let mut builder = CircuitBuilder::<F, D>::new(config);

    let x = builder.add_virtual_target();
    let x_pow_n = builder.exp_u64(x, N);

    builder.register_public_input(x_pow_n);

    builder.add_simple_generator(NthRootGenerator::<F, D, N> {
        x,
        x_pow_n,
        _phantom: PhantomData,
    });

    use plonky2_field::types::Field;

    // randomly generate a N-th root of x
    let euler = F::NEG_ONE.to_canonical_biguint() / N;
    let x_pow_n_value = {
        let mut val = F::rand();
        while val.exp_biguint(&euler) != F::ONE {
            val = F::rand();
        }
        val
    };

    let mut pw = PartialWitness::new();
    pw.set_target(x_pow_n, x_pow_n_value);

    let data = builder.build::<C>();
    let proof = data.prove(pw.clone())?;

    let x_pow_n_actual = proof.public_inputs[0];
    println!("Field element N-th power: {x_pow_n_actual}");

    data.verify(proof)
}
