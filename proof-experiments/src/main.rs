use anyhow::Result;
use plonky2::iop::witness::{PartialWitness, WitnessWrite};
use plonky2::field::types::Field;
use plonky2::plonk::circuit_data::CircuitConfig;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

// replay fibonacci with Plonky2
fn main() -> Result<()> {
    println!("Hello, world!");

    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    let config: CircuitConfig = CircuitConfig::standard_recursion_config();
    let mut builder = CircuitBuilder::<F, D>::new(config);

    let initial_a = builder.add_virtual_target();
    let initial_b = builder.add_virtual_target();
    let mut prev_target = initial_a;
    let mut cur_target = initial_b;
    for _ in 0..99999 {
        let temp = builder.add(prev_target, cur_target);
        prev_target = cur_target;
        cur_target = temp;
    }

    // the public inputs are the two initial values provided below and the result
    builder.register_public_input(initial_a);
    builder.register_public_input(initial_b);
    builder.register_public_input(cur_target);

    // provide the initial values
    let mut partial_witness = PartialWitness::new();
    partial_witness.set_target(initial_a, F::ZERO);
    partial_witness.set_target(initial_b, F::ONE);
    builder.register_public_input(cur_target);

    use std::time::Instant;
    let now = Instant::now();

    // build circuit and prove
    let data = builder.build::<C>();
    let proof = data.prove(partial_witness)?;
    println!("done proving, elapsed: {:.2?}", now.elapsed());

    println!(
        "100th Fibonacci number mod |F| (starting with {}, {}) is: {}",
        proof.public_inputs[0], proof.public_inputs[1], proof.public_inputs[2]
    );

    data.verify(proof)
}
