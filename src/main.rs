use halo2_proofs::{
    circuit::{AssignedCell, Chip, Layouter, SimpleFloorPlanner, Value},
    dev::MockProver,
    pasta::Fp,
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Instance, TableColumn},
    poly::Rotation,
};

pub struct AddChip {
    config: AddChipConfig,
}

#[derive(Clone, Debug)]
pub struct AddChipConfig {
    a_col: Column<Advice>,
    b_col: Column<Advice>,
    c_col: Column<Advice>,
    add_a_col: TableColumn,
    add_b_col: TableColumn,
    add_c_col: TableColumn,
    pub_col: Column<Instance>,
}

impl Chip<Fp> for AddChip {
    type Config = AddChipConfig;
    type Loaded = ();

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn loaded(&self) -> &Self::Loaded {
        &()
    }
}

impl AddChip {
    pub fn new(config: AddChipConfig) -> Self {
        Self { config }
    }

    pub fn configure(meta: &mut ConstraintSystem<Fp>) -> AddChipConfig {
        let a = meta.advice_column();
        let b = meta.advice_column();
        let c = meta.advice_column();
        let pub_col = meta.instance_column();
        let add_a_col = meta.lookup_table_column();
        let add_b_col = meta.lookup_table_column();
        let add_c_col = meta.lookup_table_column();
        
        meta.enable_equality(a);
        meta.enable_equality(b);
        meta.enable_equality(c);
        meta.enable_equality(pub_col);

        meta.lookup(|meta| {
            vec![(meta.query_advice(a, Rotation::cur()), add_a_col),
            (meta.query_advice(b, Rotation::cur()), add_b_col),
            (meta.query_advice(c, Rotation::cur()), add_c_col),]
        });

        AddChipConfig {
            a_col: a,
            b_col: b,
            c_col: c,
            add_a_col,
            add_b_col,
            add_c_col,
            pub_col,
        }

    }

    fn alloc_table(&self, layouter: &mut impl Layouter<Fp>) -> Result<(), Error> {
        layouter.assign_table(
            || "add table(range 0-10)",
            |mut table| {
                let mut row_offset = 0;

                // Every combination of input-output pairs 
                for n in 0..=10 {
                    for nn in 0..=10 {
                        table.assign_cell(
                            || format!("add_a_col row {}", row_offset),
                            self.config.add_a_col,
                            row_offset,
                            || Value::known(Fp::from(n)),
                        )?;

                        table.assign_cell(
                            || format!("add_b_col row {}", row_offset),
                            self.config.add_b_col,
                            row_offset,
                            || Value::known(Fp::from(nn)),
                        )?;

                        table.assign_cell(
                            || format!("add_c_col row {}", row_offset),
                            self.config.add_c_col,
                            row_offset,
                            || Value::known(Fp::from(n + nn)),
                        )?;
                        row_offset += 1;
                    }
                }
                Ok(())
            },
        )
    }

    fn alloc_private_and_public_inputs(&self, layouter: &mut impl Layouter<Fp>, a: Fp, b: Fp, c: Fp) -> Result<AssignedCell<Fp, Fp>, Error> {
        let a = Value::known(a);
        let b = Value::known(b);
        let c = Value::known(c);
        layouter.assign_region(
            || "public and private inputs",
            |mut region| {
                let row_offset = 0;
                region.assign_advice(|| "private input a",
                self.config.a_col,
                row_offset,
                || a)?;

                region.assign_advice(
                    || "private input b",
                    self.config.b_col,
                    row_offset,
                    || b,
                )?;

                let c = region.assign_advice(
                    || "private input c",
                    self.config.c_col,
                    row_offset,
                    || c,
                )?;

                Ok(c)
            },
        )
    }
}

#[derive(Clone)]
struct AddCircuit {
    a: Fp,
    b: Fp,
    c: Fp,
}

impl Circuit<Fp> for AddCircuit {
    type Config = AddChipConfig;

    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        todo!()
    }

    fn configure(cs: &mut ConstraintSystem<Fp>) -> Self::Config {
        AddChip::configure(cs)
    }

    fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<Fp>,) -> Result<(), Error> {
        let add_chip = AddChip::new(config);
        add_chip.alloc_table(&mut layouter)?;
        let c = add_chip.alloc_private_and_public_inputs(&mut layouter, self.a, self.b, self.c)?;

        layouter.constrain_instance(c.cell(), add_chip.config().pub_col, 0)
    }
}

fn main() {
    let pub_input = 15;
    // 2^k rows
    // why 7?
    // Number of rows should be as big as lookup table size.
    // In our case, lookup table is about 121 rows (11 * 11)
    // So, we have the layout of next power of 121 which is 128. 
    let k = 7;

    let pub_inputs = vec![Fp::from(pub_input)];

    let circuit = AddCircuit {
        a: Fp::from(7),
        b: Fp::from(8),
        c: Fp::from(pub_input),
    };

    let prover = MockProver::run(k, &circuit, vec![pub_inputs.clone()]).unwrap();

    assert!(prover.verify().is_ok());

    // wrong public input
    let pub_inputs = vec![Fp::from(0)];

    let prover = MockProver::run(k, &circuit, vec![pub_inputs.clone()]).unwrap();
    
    // Should fail with an error
    assert!(prover.verify().is_err());   
}