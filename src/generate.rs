#![allow(dead_code, unused_imports)]

use ark_bls12_381::{Fr, G1Affine, G1Projective};
use ark_ec::{AffineCurve, ProjectiveCurve};
use ark_ff::*;
use ark_poly::{
    univariate::DensePolynomial, EvaluationDomain, GeneralEvaluationDomain, Polynomial,
    UVPolynomial,
};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};

fn generate_kzg_setup(n: usize) -> Vec<G1Affine> {
    let mut rng = rand::thread_rng();
    let secret_scalar = Fr::rand(&mut rng);
    let secret_powers: Vec<Fr> = (0..n as u64)
        .map(|p| secret_scalar.pow(&[p, 0, 0, 0]))
        .collect();
    let generator = G1Projective::prime_subgroup_generator();
    let kzg_setup: Vec<G1Affine> = secret_powers
        .iter()
        .map(|s| (generator.mul(s.into_repr())).into_affine())
        .collect();

    kzg_setup
}

pub fn kzg_commit(p: &DensePolynomial<Fr>, setup: &Vec<G1Affine>) -> G1Affine {
    p.coeffs()
        .iter()
        .zip(setup)
        .map(|(c, p)| p.into_projective().mul(c.into_repr()))
        .sum::<G1Projective>()
        .into_affine()
}

fn generate_acct() -> Vec<Fr> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let acct_bytes: Vec<u8> = (0..32).into_iter().map(|_u| rng.gen::<u8>()).collect();
    acct_bytes.iter().map(|u| Fr::from(*u)).collect()
}

fn generate_accts(n: usize) -> Vec<Vec<Fr>> {
    (0..n)
        .into_iter()
        .map(|_u| generate_acct())
        .collect::<Vec<_>>()
}

pub fn generate_challenge() -> (Vec<G1Affine>, Vec<Vec<Fr>>, Fr, Fr, G1Affine, Fr, Fr) {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    let number_of_accts = 1000usize;
    let accts = generate_accts(number_of_accts);
    let target_acct_index = rng.gen_range(0..number_of_accts);
    let target_acct = &accts[target_acct_index];

    println!("acct_{} ({}) = {:?}", target_acct_index, target_acct.len(), target_acct);

    let domain: GeneralEvaluationDomain<Fr> =
        GeneralEvaluationDomain::new(number_of_accts + 2).unwrap();
    let setup = generate_kzg_setup(domain.size());

    println!("setup {:?} ({})", setup, setup.len());
    println!("num_accounts = {}", number_of_accts);

    let target_acct_poly = DensePolynomial::from_coefficients_vec(domain.ifft(&target_acct));
    let blinding_poly =
        DensePolynomial::from_coefficients_vec(vec![Fr::rand(&mut rng), Fr::rand(&mut rng)]);
    let blinded_acct_poly = target_acct_poly.clone() + blinding_poly.mul_by_vanishing_poly(domain);

    let vanish =  DensePolynomial::from_coefficients_vec(domain.ifft(&vec![Fr::from(0); domain.size()]));//.mul_by_vanishing_poly(domain);


    let domain2: GeneralEvaluationDomain<Fr> =
        GeneralEvaluationDomain::new(number_of_accts * 2).unwrap();

    let blinded_evals = blinded_acct_poly.clone().evaluate_over_domain(domain).evals;
    let target_evals = target_acct_poly.clone().evaluate_over_domain(domain).evals;
    let blinding_evals = blinding_poly.mul_by_vanishing_poly(domain).evaluate_over_domain(domain).evals;


    let tmp_poly = blinded_acct_poly.divide_by_vanishing_poly(domain).unwrap().0;
    assert_eq!(tmp_poly, blinding_poly);
    

    for (i,(e, t)) in target_evals.iter().zip(target_acct).enumerate() {
        println!("{} {:?}\n  {:?}", i, e, t);
    }

    for (i,e) in blinding_evals.iter().enumerate() {
        println!("{} {:?}", i, e);
    }

    println!("{} {} {} {}", target_acct_poly.degree(), blinding_poly.degree(), blinded_acct_poly.degree(), &vanish.degree());


    for acct in accts.clone() {
    //    println!("{:?}", acct);
    }

    let commitment: G1Affine = kzg_commit(&blinded_acct_poly, &setup);

    let challenge_1 = Fr::rand(&mut rng);
    let challenge_2 = Fr::rand(&mut rng);

    let opening_1 = blinded_acct_poly.evaluate(&challenge_1);
    let opening_2 = blinded_acct_poly.evaluate(&challenge_2);

    let p1 = target_acct_poly.evaluate(&challenge_1);
    let v1 = domain.vanishing_polynomial().evaluate(&challenge_1);
    let tmp1 = (opening_1 - p1) * v1.inverse().unwrap();

    let p2 = target_acct_poly.evaluate(&challenge_2);
    let v2 = domain.vanishing_polynomial().evaluate(&challenge_2);
    let tmp2 = (opening_2 - p2) / v2;
    //println!("{:?}", tmp2);
    //println!("{:?}", blinding_poly.evaluate(&challenge_2));

    let b1 = (tmp1 - tmp2)/(challenge_1 - challenge_2);
    let b0 = tmp1 - challenge_1 * b1;
    println!("{:?}", b1);
    println!("{:?}", blinding_poly.coeffs[1]);
    let recovd_b_poly = DensePolynomial::from_coefficients_vec(vec![b0, b1]);
    assert_eq!(recovd_b_poly, blinding_poly);
    println!("eq!");


    (
        setup,
        accts,
        challenge_1,
        challenge_2,
        commitment,
        opening_1,
        opening_2,
    )
}
