use std::{
    fmt::Debug,
    ops::{Mul, Neg},
};

use ark_bn254::{Bn254, Fq, Fr, G1Affine, G2Affine};
use ark_ec::{pairing::Pairing, AffineRepr, CurveGroup};
use ark_ff::{BigInt, BigInteger, Field, One, PrimeField, UniformRand, Zero};
use const_hex::ToHexExt;
use ethers::types::{H256, U256};
use rand_core::CryptoRngCore;
use zeroize::ZeroizeOnDrop;

use crate::{
    crypto::signer::Signer,
    hash::{
        keccak256::{Keccak256Hash, Keccak256Hasher},
        Hashable,
    },
};

pub trait FqConvert {
    fn to_u256(&self) -> U256;
    fn from_u256(u: U256) -> Self;
}

pub trait G1AConvert {
    fn to_bytes_be(&self) -> Vec<u8>;
    fn to_bytes_le(&self) -> Vec<u8>;
    fn to_u256_points(&self) -> (U256, U256);
    fn from_u256_points(x: U256, y: U256) -> Self;
}

pub trait G2AConvert {
    fn to_u256_points(&self) -> ([U256; 2], [U256; 2]);
    fn to_bytes_le(&self) -> Vec<u8>;
}

impl FqConvert for Fq {
    fn to_u256(&self) -> U256 {
        U256::from_little_endian(&self.into_bigint().to_bytes_le())
    }

    fn from_u256(u: U256) -> Self {
        let mut bytes = [0u8; 32];
        u.to_little_endian(&mut bytes);
        Self::from_le_bytes_mod_order(&bytes)
    }
}

impl G1AConvert for G1Affine {
    fn to_bytes_be(&self) -> Vec<u8> {
        let xy = self.xy().expect("affine should have XY");
        [
            xy.0.into_bigint().to_bytes_be(),
            xy.1.into_bigint().to_bytes_be(),
        ]
        .concat()
        .to_vec()
    }

    fn to_bytes_le(&self) -> Vec<u8> {
        let xy = self.xy().expect("affine should have XY");
        [
            xy.0.into_bigint().to_bytes_le(),
            xy.1.into_bigint().to_bytes_le(),
        ]
        .concat()
        .to_vec()
    }

    fn to_u256_points(&self) -> (U256, U256) {
        let (x, y) = self.xy().expect("affine should have XY");
        (x.to_u256(), y.to_u256())
    }

    fn from_u256_points(x: U256, y: U256) -> Self {
        G1Affine::new(Fq::from_u256(x), Fq::from_u256(y))
    }
}

impl G2AConvert for G2Affine {
    fn to_u256_points(&self) -> ([U256; 2], [U256; 2]) {
        let (x, y) = self.xy().expect("affine should have XY");
        (
            [x.c1.to_u256(), x.c0.to_u256()],
            [y.c1.to_u256(), y.c0.to_u256()],
        )
    }

    fn to_bytes_le(&self) -> Vec<u8> {
        let (x, y) = self.to_u256_points();
        let mut out: Vec<u8> = vec![];
        let mut bytes = [0u8; 32];

        x[0].to_little_endian(&mut bytes);
        out.extend_from_slice(&bytes);

        x[1].to_little_endian(&mut bytes);
        out.extend_from_slice(&bytes);

        y[0].to_little_endian(&mut bytes);
        out.extend_from_slice(&bytes);

        y[1].to_little_endian(&mut bytes);
        out.extend_from_slice(&bytes);

        out
    }
}

#[derive(thiserror::Error, Debug)]
pub enum BlsSignerError {
    #[error("hex error: {0}")]
    FromHex(#[from] const_hex::FromHexError),
    #[error("signature verification error")]
    SignatureVerification,
}

#[derive(ZeroizeOnDrop, Clone)]
pub struct BlsSigner {
    wallet: Fr,
    #[zeroize(skip)]
    public_key: G1Affine,
}

impl Debug for BlsSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "BlsSigner {{ address: {:?} }}",
            self.hash::<Keccak256Hasher>().encode_hex_with_prefix()
        ))
    }
}

impl Hashable for BlsSigner {
    fn collect(&self) -> std::borrow::Cow<[u8]> {
        self.public_key.to_bytes_be().into()
    }
}

impl BlsSigner {
    /// BN254 map to curve from
    /// contracts/lib/eigenlayer-middleware/lib/eigenlayer-contracts/src/contracts/libraries/BN254.sol
    /// for a hash, maps to a point on curve
    /// y^2 = x^3 + b
    fn map_to_curve(hash: &[u8]) -> G1Affine {
        let mut x: Fq = Fq::from_be_bytes_mod_order(hash);
        let b = BigInt::<4>::from(3_u32);

        loop {
            let beta = x.pow(b) + Fq::from(3_u32);
            if let Some(y) = beta.sqrt() {
                return G1Affine::new(x, y);
            } else {
                x += Fq::one()
            }
        }
    }

    pub fn sign_hashed(&self, g1: G1Affine) -> G1Affine {
        g1.mul(self.wallet).into_affine()
    }
}

impl Signer for BlsSigner {
    type PrivateKey = Fr;
    type PublicKey = G1Affine;
    type VerifyingKey = G2Affine;
    type Signature = G1Affine;
    type Hasher = Keccak256Hasher;
    type Hash = Keccak256Hash;
    type SignerError = BlsSignerError;

    fn from_key(key: Self::PrivateKey) -> Self {
        let public_key = G1Affine::generator().mul(key).into_affine();
        Self {
            wallet: key,
            public_key,
        }
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::SignerError>
    where
        Self: Sized,
    {
        let key = Fr::from_le_bytes_mod_order(bytes);
        Ok(Self::from_key(key))
    }

    fn from_random(mut rng: impl CryptoRngCore) -> Result<(Self, Vec<u8>), Self::SignerError>
    where
        Self: Sized,
    {
        let key = Fr::rand(&mut rng);
        Ok((Self::from_key(key), key.0.to_bytes_le()))
    }

    fn hash_and_sign<D: Hashable>(&self, data: D) -> Result<Self::Signature, Self::SignerError> {
        let data_hash = H256::from_slice(data.hash::<Self::Hasher>().as_ref());
        let affine = Self::map_to_curve(data_hash.as_bytes());
        Ok((affine * self.wallet).into_affine())
    }

    fn sign(&self, data: &[u8]) -> Result<Self::Signature, Self::SignerError> {
        let affine = Self::map_to_curve(data);
        Ok(affine.mul(self.wallet).into_affine())
    }

    fn public_key_vec(&self) -> Vec<u8> {
        self.public_key().to_bytes_le()
    }

    fn verifying_key_vec(&self) -> Vec<u8> {
        self.verifying_key().to_bytes_le()
    }

    fn public_key(&self) -> Self::PublicKey {
        self.public_key
    }

    fn verifying_key(&self) -> Self::VerifyingKey {
        G2Affine::generator().mul(self.wallet).into_affine()
    }

    fn verify(
        hash: &Self::Hash,
        pubkey: &Self::VerifyingKey,
        signature: &Self::Signature,
    ) -> Result<(), Self::SignerError> {
        let hash_point = Self::map_to_curve(hash.as_ref());
        let neg_sig = signature.neg();

        let g2_gen = G2Affine::generator();

        let p = [hash_point, neg_sig];
        let q = [*pubkey, g2_gen];

        let pairing = Bn254::multi_pairing(p.iter(), q.iter());

        if pairing.is_zero() {
            Ok(())
        } else {
            Err(BlsSignerError::SignatureVerification)
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::rngs::StdRng;
    use rand_core::SeedableRng;

    use super::*;

    struct TestHashable {
        data: String,
    }

    impl Hashable for TestHashable {
        fn collect(&self) -> std::borrow::Cow<[u8]> {
            self.data.as_bytes().into()
        }
    }

    #[test]
    fn test_bls_signer() {
        let key = Fr::rand(&mut StdRng::seed_from_u64(0));
        let signer = BlsSigner::from_key(key);
        let data = TestHashable {
            data: "hello".to_string(),
        };
        let hash = data.hash::<Keccak256Hasher>();
        let signature = signer.hash_and_sign(data).unwrap();
        assert!(BlsSigner::verify(&hash, &signer.verifying_key(), &signature).is_ok());
    }
}
