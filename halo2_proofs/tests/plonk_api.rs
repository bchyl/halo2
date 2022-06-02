#![allow(clippy::many_single_char_names)]
#![allow(clippy::op_ref)]

use halo2_proofs::arithmetic::Field;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::{Cell, Layouter, SimpleFloorPlanner};
use halo2_proofs::dev::MockProver;
use halo2_proofs::plonk::{
    create_proof as create_plonk_proof, keygen_pk, keygen_vk, verify_proof as verify_plonk_proof,
    Advice, Circuit, Column, ConstraintSystem, Error, Fixed, ProvingKey, TableColumn, VerifyingKey,
};
use halo2_proofs::poly::commitment::{
    CommitmentScheme, ParamsProver, Prover as _Prover, Verifier as _Verifier,
};
use halo2_proofs::poly::Rotation;
use halo2_proofs::poly::VerificationStrategy;
use halo2_proofs::transcript::{
    Blake2bRead, Blake2bWrite, Challenge255, EncodedChallenge, TranscriptReadBuffer,
    TranscriptWriterBuffer,
};

use rand_core::{OsRng, RngCore};
use std::marker::PhantomData;

#[test]
fn plonk_api() {
    const K: u32 = 5;

    /// This represents an advice column at a certain row in the ConstraintSystem
    #[derive(Copy, Clone, Debug)]
    pub struct Variable(Column<Advice>, usize);

    #[derive(Clone)]
    struct PlonkConfig {
        a: Column<Advice>,
        b: Column<Advice>,
        c: Column<Advice>,
        d: Column<Advice>,
        e: Column<Advice>,

        sa: Column<Fixed>,
        sb: Column<Fixed>,
        sc: Column<Fixed>,
        sm: Column<Fixed>,
        sp: Column<Fixed>,
        sl: TableColumn,
    }

    #[allow(clippy::type_complexity)]
    trait StandardCs<FF: FieldExt> {
        fn raw_multiply<F>(
            &self,
            layouter: &mut impl Layouter<FF>,
            f: F,
        ) -> Result<(Cell, Cell, Cell), Error>
        where
            F: FnMut() -> Result<(FF, FF, FF), Error>;
        fn raw_add<F>(
            &self,
            layouter: &mut impl Layouter<FF>,
            f: F,
        ) -> Result<(Cell, Cell, Cell), Error>
        where
            F: FnMut() -> Result<(FF, FF, FF), Error>;
        fn copy(&self, layouter: &mut impl Layouter<FF>, a: Cell, b: Cell) -> Result<(), Error>;
        fn public_input<F>(&self, layouter: &mut impl Layouter<FF>, f: F) -> Result<Cell, Error>
        where
            F: FnMut() -> Result<FF, Error>;
        fn lookup_table(
            &self,
            layouter: &mut impl Layouter<FF>,
            values: &[FF],
        ) -> Result<(), Error>;
    }

    #[derive(Clone)]
    struct MyCircuit<F: FieldExt> {
        a: Option<F>,
        lookup_table: Vec<F>,
    }

    // impl<F: FieldExt> MyCircuit<F> {
    //     fn new(a: F) -> Self {
    //         Self { a }
    //     }
    // }

    struct StandardPlonk<F: FieldExt> {
        config: PlonkConfig,
        _marker: PhantomData<F>,
    }

    impl<FF: FieldExt> StandardPlonk<FF> {
        fn new(config: PlonkConfig) -> Self {
            StandardPlonk {
                config,
                _marker: PhantomData,
            }
        }
    }

    impl<FF: FieldExt> StandardCs<FF> for StandardPlonk<FF> {
        fn raw_multiply<F>(
            &self,
            layouter: &mut impl Layouter<FF>,
            mut f: F,
        ) -> Result<(Cell, Cell, Cell), Error>
        where
            F: FnMut() -> Result<(FF, FF, FF), Error>,
        {
            layouter.assign_region(
                || "raw_multiply",
                |mut region| {
                    let mut value = None;
                    let lhs = region.assign_advice(
                        || "lhs",
                        self.config.a,
                        0,
                        || {
                            value = Some(f()?);
                            Ok(value.ok_or(Error::Synthesis)?.0)
                        },
                    )?;
                    region.assign_advice(
                        || "lhs^4",
                        self.config.d,
                        0,
                        || Ok(value.ok_or(Error::Synthesis)?.0.square().square()),
                    )?;
                    let rhs = region.assign_advice(
                        || "rhs",
                        self.config.b,
                        0,
                        || Ok(value.ok_or(Error::Synthesis)?.1),
                    )?;
                    region.assign_advice(
                        || "rhs^4",
                        self.config.e,
                        0,
                        || Ok(value.ok_or(Error::Synthesis)?.1.square().square()),
                    )?;
                    let out = region.assign_advice(
                        || "out",
                        self.config.c,
                        0,
                        || Ok(value.ok_or(Error::Synthesis)?.2),
                    )?;

                    region.assign_fixed(|| "a", self.config.sa, 0, || Ok(FF::zero()))?;
                    region.assign_fixed(|| "b", self.config.sb, 0, || Ok(FF::zero()))?;
                    region.assign_fixed(|| "c", self.config.sc, 0, || Ok(FF::one()))?;
                    region.assign_fixed(|| "a * b", self.config.sm, 0, || Ok(FF::one()))?;
                    Ok((lhs.cell(), rhs.cell(), out.cell()))
                },
            )
        }
        fn raw_add<F>(
            &self,
            layouter: &mut impl Layouter<FF>,
            mut f: F,
        ) -> Result<(Cell, Cell, Cell), Error>
        where
            F: FnMut() -> Result<(FF, FF, FF), Error>,
        {
            layouter.assign_region(
                || "raw_add",
                |mut region| {
                    let mut value = None;
                    let lhs = region.assign_advice(
                        || "lhs",
                        self.config.a,
                        0,
                        || {
                            value = Some(f()?);
                            Ok(value.ok_or(Error::Synthesis)?.0)
                        },
                    )?;
                    region.assign_advice(
                        || "lhs^4",
                        self.config.d,
                        0,
                        || Ok(value.ok_or(Error::Synthesis)?.0.square().square()),
                    )?;
                    let rhs = region.assign_advice(
                        || "rhs",
                        self.config.b,
                        0,
                        || Ok(value.ok_or(Error::Synthesis)?.1),
                    )?;
                    region.assign_advice(
                        || "rhs^4",
                        self.config.e,
                        0,
                        || Ok(value.ok_or(Error::Synthesis)?.1.square().square()),
                    )?;
                    let out = region.assign_advice(
                        || "out",
                        self.config.c,
                        0,
                        || Ok(value.ok_or(Error::Synthesis)?.2),
                    )?;

                    region.assign_fixed(|| "a", self.config.sa, 0, || Ok(FF::one()))?;
                    region.assign_fixed(|| "b", self.config.sb, 0, || Ok(FF::one()))?;
                    region.assign_fixed(|| "c", self.config.sc, 0, || Ok(FF::one()))?;
                    region.assign_fixed(|| "a * b", self.config.sm, 0, || Ok(FF::zero()))?;
                    Ok((lhs.cell(), rhs.cell(), out.cell()))
                },
            )
        }
        fn copy(
            &self,
            layouter: &mut impl Layouter<FF>,
            left: Cell,
            right: Cell,
        ) -> Result<(), Error> {
            layouter.assign_region(
                || "copy",
                |mut region| {
                    region.constrain_equal(left, right)?;
                    region.constrain_equal(left, right)
                },
            )
        }
        fn public_input<F>(&self, layouter: &mut impl Layouter<FF>, mut f: F) -> Result<Cell, Error>
        where
            F: FnMut() -> Result<FF, Error>,
        {
            layouter.assign_region(
                || "public_input",
                |mut region| {
                    let value = region.assign_advice(|| "value", self.config.a, 0, &mut f)?;
                    region.assign_fixed(|| "public", self.config.sp, 0, || Ok(FF::one()))?;

                    Ok(value.cell())
                },
            )
        }
        fn lookup_table(
            &self,
            layouter: &mut impl Layouter<FF>,
            values: &[FF],
        ) -> Result<(), Error> {
            layouter.assign_table(
                || "",
                |mut table| {
                    for (index, &value) in values.iter().enumerate() {
                        table.assign_cell(|| "table col", self.config.sl, index, || Ok(value))?;
                    }
                    Ok(())
                },
            )?;
            Ok(())
        }
    }

    impl<F: FieldExt> Circuit<F> for MyCircuit<F> {
        type Config = PlonkConfig;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            Self {
                a: None,
                lookup_table: self.lookup_table.clone(),
            }
        }

        fn configure(meta: &mut ConstraintSystem<F>) -> PlonkConfig {
            let e = meta.advice_column();
            let a = meta.advice_column();
            let b = meta.advice_column();
            let sf = meta.fixed_column();
            let c = meta.advice_column();
            let d = meta.advice_column();
            let p = meta.instance_column();

            meta.enable_equality(a);
            meta.enable_equality(b);
            meta.enable_equality(c);

            let sm = meta.fixed_column();
            let sa = meta.fixed_column();
            let sb = meta.fixed_column();
            let sc = meta.fixed_column();
            let sp = meta.fixed_column();
            let sl = meta.lookup_table_column();

            /*
             *   A         B      ...  sl
             * [
             *   instance  0      ...  0
             *   a         a      ...  0
             *   a         a^2    ...  0
             *   a         a      ...  0
             *   a         a^2    ...  0
             *   ...       ...    ...  ...
             *   ...       ...    ...  instance
             *   ...       ...    ...  a
             *   ...       ...    ...  a
             *   ...       ...    ...  0
             * ]
             */

            meta.lookup(|meta| {
                let a_ = meta.query_any(a, Rotation::cur());
                vec![(a_, sl)]
            });

            meta.create_gate("Combined add-mult", |meta| {
                let d = meta.query_advice(d, Rotation::next());
                let a = meta.query_advice(a, Rotation::cur());
                let sf = meta.query_fixed(sf, Rotation::cur());
                let e = meta.query_advice(e, Rotation::prev());
                let b = meta.query_advice(b, Rotation::cur());
                let c = meta.query_advice(c, Rotation::cur());

                let sa = meta.query_fixed(sa, Rotation::cur());
                let sb = meta.query_fixed(sb, Rotation::cur());
                let sc = meta.query_fixed(sc, Rotation::cur());
                let sm = meta.query_fixed(sm, Rotation::cur());

                vec![a.clone() * sa + b.clone() * sb + a * b * sm - (c * sc) + sf * (d * e)]
            });

            meta.create_gate("Public input", |meta| {
                let a = meta.query_advice(a, Rotation::cur());
                let p = meta.query_instance(p, Rotation::cur());
                let sp = meta.query_fixed(sp, Rotation::cur());

                vec![sp * (a - p)]
            });

            meta.enable_equality(sf);
            meta.enable_equality(e);
            meta.enable_equality(d);
            meta.enable_equality(p);
            meta.enable_equality(sm);
            meta.enable_equality(sa);
            meta.enable_equality(sb);
            meta.enable_equality(sc);
            meta.enable_equality(sp);

            PlonkConfig {
                a,
                b,
                c,
                d,
                e,
                sa,
                sb,
                sc,
                sm,
                sp,
                sl,
            }
        }

        fn synthesize(
            &self,
            config: PlonkConfig,
            mut layouter: impl Layouter<F>,
        ) -> Result<(), Error> {
            let cs = StandardPlonk::new(config);

            let _ = cs.public_input(&mut layouter, || Ok(F::one() + F::one()))?;

            for _ in 0..10 {
                let mut a_squared = None;
                let (a0, _, c0) = cs.raw_multiply(&mut layouter, || {
                    a_squared = self.a.map(|a| a.square());
                    Ok((
                        self.a.ok_or(Error::Synthesis)?,
                        self.a.ok_or(Error::Synthesis)?,
                        a_squared.ok_or(Error::Synthesis)?,
                    ))
                })?;
                let (a1, b1, _) = cs.raw_add(&mut layouter, || {
                    let fin = a_squared.and_then(|a2| self.a.map(|a| a + a2));
                    Ok((
                        self.a.ok_or(Error::Synthesis)?,
                        a_squared.ok_or(Error::Synthesis)?,
                        fin.ok_or(Error::Synthesis)?,
                    ))
                })?;
                cs.copy(&mut layouter, a0, a1)?;
                cs.copy(&mut layouter, b1, c0)?;
            }

            cs.lookup_table(&mut layouter, &self.lookup_table)?;

            Ok(())
        }
    }

    macro_rules! common {
        () => {{
            let a = Scheme::Scalar::from(2834758237) * Scheme::Scalar::ZETA;
            let instance = Scheme::Scalar::one() + Scheme::Scalar::one();
            let lookup_table = vec![instance, a, a, Scheme::Scalar::zero()];
            (a, instance, lookup_table)
        }};
    }

    fn synthesize<'params, Scheme: CommitmentScheme<'params>>(
        params: &'params Scheme::ParamsProver,
    ) -> ProvingKey<Scheme::Curve> {
        let (_, _, lookup_table) = common!();
        let empty_circuit: MyCircuit<Scheme::Scalar> = MyCircuit {
            a: None,
            lookup_table: lookup_table.clone(),
        };
        // Initialize the proving key
        let vk =
            keygen_vk::<Scheme, _>(&params, &empty_circuit).expect("keygen_vk should not fail");

        keygen_pk::<Scheme, _>(&params, vk, &empty_circuit).expect("keygen_pk should not fail")
    }

    fn create_proof<
        'params,
        Scheme: CommitmentScheme<'params>,
        TranscriptWrite: TranscriptWriterBuffer<Vec<u8>, Scheme::Curve, Ch>,
        Prover: _Prover<'params, Scheme, Rng>,
        Ch,
        Rng,
    >(
        rng: Rng,
        params: &'params Scheme::ParamsProver,
        pk: &ProvingKey<Scheme::Curve>,
    ) -> Vec<u8>
    where
        Ch: EncodedChallenge<Scheme::Curve>,
        Rng: RngCore,
    {
        let (a, instance, lookup_table) = common!();

        let circuit: MyCircuit<Scheme::Scalar> = MyCircuit {
            a: Some(a),
            lookup_table,
        };

        let mut transcript = TranscriptWrite::init(vec![]);

        create_plonk_proof::<Scheme, Prover, _, _, _, _>(
            &params,
            &pk,
            &[circuit.clone(), circuit.clone()],
            &[&[&[instance]], &[&[instance]]],
            rng,
            &mut transcript,
        )
        .expect("proof generation should not fail");

        // Check this circuit is satisfied.
        let prover = match MockProver::run(K, &circuit, vec![vec![instance]]) {
            Ok(prover) => prover,
            Err(e) => panic!("{:?}", e),
        };
        assert_eq!(prover.verify(), Ok(()));

        transcript.finalize()
    }

    fn verify_proof<
        'a,
        'params,
        Scheme: CommitmentScheme<'params>,
        Verifier: _Verifier<'params, Scheme>,
        TranscriptRead: TranscriptReadBuffer<&'a [u8], Scheme::Curve, Ch>,
        Strategy: VerificationStrategy<'params, Scheme, Verifier, Rng, Output = Strategy>,
        Ch,
        Rng,
    >(
        rng: Rng,
        params_verifier: &'params Scheme::ParamsVerifier,
        vk: &VerifyingKey<Scheme::Curve>,
        proof: &'a [u8],
    ) where
        Ch: EncodedChallenge<Scheme::Curve>,
        Rng: RngCore + Copy,
    {
        let (_, instance, _) = common!();
        let pubinputs = vec![instance];

        let mut transcript = TranscriptRead::init(&proof[..]);

        let strategy = Strategy::new(&params_verifier, rng);
        let strategy = verify_plonk_proof(
            params_verifier,
            vk,
            strategy,
            &[&[&pubinputs[..]], &[&pubinputs[..]]],
            &mut transcript,
        )
        .unwrap();

        assert!(strategy.finalize());
    }

    fn test_plonk_api_ipa() {
        use halo2_proofs::poly::ipa::commitment::{IPACommitmentScheme, ParamsIPA};
        use halo2_proofs::poly::ipa::multiopen::{ProverIPA, VerifierIPA};
        use halo2_proofs::poly::ipa::strategy::BatchVerifier;
        use halo2curves::pasta::EqAffine;

        let params = ParamsIPA::<EqAffine>::new(K);
        let rng = OsRng;

        let pk = synthesize::<IPACommitmentScheme<EqAffine>>(&params);

        let proof = create_proof::<_, Blake2bWrite<_, _, Challenge255<_>>, ProverIPA<_>, _, _>(
            rng, &params, &pk,
        );

        let verifier_params = params.verifier_params();

        verify_proof::<
            _,
            VerifierIPA<_>,
            Blake2bRead<_, _, Challenge255<_>>,
            BatchVerifier<_, _>,
            _,
            _,
        >(rng, &verifier_params, pk.get_vk(), &proof[..]);
    }

    fn test_plonk_api_gwc() {
        use halo2_proofs::poly::kzg::commitment::{KZGCommitmentScheme, ParamsKZG};
        use halo2_proofs::poly::kzg::multiopen::{ProverGWC, VerifierGWC};
        use halo2_proofs::poly::kzg::strategy::BatchVerifier;
        use halo2curves::bn256::Bn256;

        let params = ParamsKZG::<Bn256>::new(K);
        let rng = OsRng;

        let pk = synthesize::<KZGCommitmentScheme<_>>(&params);

        let proof = create_proof::<_, Blake2bWrite<_, _, Challenge255<_>>, ProverGWC<_>, _, _>(
            rng, &params, &pk,
        );

        let verifier_params = params.verifier_params();

        verify_proof::<
            _,
            VerifierGWC<_>,
            Blake2bRead<_, _, Challenge255<_>>,
            BatchVerifier<_, _>,
            _,
            _,
        >(rng, &verifier_params, pk.get_vk(), &proof[..]);
    }

    fn test_plonk_api_shplonk() {
        use halo2_proofs::poly::kzg::commitment::{KZGCommitmentScheme, ParamsKZG};
        use halo2_proofs::poly::kzg::multiopen::{ProverSHPLONK, VerifierSHPLONK};
        use halo2_proofs::poly::kzg::strategy::BatchVerifier;
        use halo2curves::bn256::Bn256;

        let params = ParamsKZG::<Bn256>::new(K);
        let rng = OsRng;

        let pk = synthesize::<KZGCommitmentScheme<_>>(&params);

        let proof = create_proof::<_, Blake2bWrite<_, _, Challenge255<_>>, ProverSHPLONK<_>, _, _>(
            rng, &params, &pk,
        );

        let verifier_params = params.verifier_params();

        verify_proof::<
            _,
            VerifierSHPLONK<_>,
            Blake2bRead<_, _, Challenge255<_>>,
            BatchVerifier<_, _>,
            _,
            _,
        >(rng, &verifier_params, pk.get_vk(), &proof[..]);
    }

    test_plonk_api_ipa();
    test_plonk_api_gwc();
    test_plonk_api_shplonk();
}
