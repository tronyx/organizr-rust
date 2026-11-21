//! The certificate of a certificate authority that an Admin uploads, so that requests to
//! the services that the Admin chose trust certificates that this authority signed. PHP
//! Organizr keeps it at `data/cert/custom.pem`; this application keeps it at
//! `cert/custom.pem` in the data folder.
//!
//! The file must be PEM text with one or more certificates and nothing else that PEM
//! can hold: no private key, no request and no list of revoked certificates. Each
//! certificate must be that of an authority: its basic constraints say so, and its key
//! usage, when it has one, lets it sign certificates. Only the certificates are kept, as
//! PEM text that this module writes again, so nothing else of the upload is stored.

use std::fmt::Write as _;

use base64::Engine as _;
use rustls_pki_types::pem::{PemObject as _, SectionKind};
use thiserror::Error;
use x509_parser::prelude::{FromDer as _, X509Certificate};

/// The largest file that is taken, in bytes.
pub const LARGEST: usize = 64 * 1024;

/// The most certificates that one file may hold.
pub const MOST_CERTIFICATES: usize = 20;

/// The place of the file in the data folder.
pub const FILE: &str = "cert/custom.pem";

/// Why an uploaded file is not taken. The texts name no part of the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum CertificateError {
    /// The file is larger than [`LARGEST`].
    #[error("The certificate file has {LARGEST} bytes at most")]
    TooLarge,
    /// The file holds a private key.
    #[error("The file holds a private key. Upload only the certificate of the authority")]
    PrivateKey,
    /// The file is no PEM text with a certificate.
    #[error("The file is no PEM file with a certificate")]
    NotPem,
    /// The file holds a PEM part that is no certificate.
    #[error("The file holds a part that is no certificate")]
    NotACertificate,
    /// A certificate cannot be read.
    #[error("A certificate of the file cannot be read")]
    Unreadable,
    /// A certificate is not that of a certificate authority.
    #[error("A certificate of the file is not that of a certificate authority")]
    NotAnAuthority,
    /// The file holds more than [`MOST_CERTIFICATES`] certificates.
    #[error("The file has {MOST_CERTIFICATES} certificates at most")]
    TooMany,
}

/// The certificates of an uploaded file that passed every check, as PEM text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomCertificate {
    pem: String,
    count: usize,
}

impl CustomCertificate {
    /// Checks a file and keeps its certificates.
    pub fn parse(file: &[u8]) -> Result<Self, CertificateError> {
        if file.len() > LARGEST {
            return Err(CertificateError::TooLarge);
        }
        // A key whose PEM label the reader below does not know, such as an encrypted
        // private key, is refused too.
        if file
            .windows(b"PRIVATE KEY-----".len())
            .any(|part| part == b"PRIVATE KEY-----")
        {
            return Err(CertificateError::PrivateKey);
        }
        let mut certificates = Vec::new();
        for section in <(SectionKind, Vec<u8>)>::pem_slice_iter(file) {
            let (kind, der) = section.map_err(|_| CertificateError::NotPem)?;
            match kind {
                SectionKind::Certificate => {}
                SectionKind::PrivateKey
                | SectionKind::RsaPrivateKey
                | SectionKind::EcPrivateKey => {
                    return Err(CertificateError::PrivateKey);
                }
                _ => return Err(CertificateError::NotACertificate),
            }
            if certificates.len() == MOST_CERTIFICATES {
                return Err(CertificateError::TooMany);
            }
            check_authority(&der)?;
            certificates.push(der);
        }
        if certificates.is_empty() {
            return Err(CertificateError::NotPem);
        }
        let mut pem = String::new();
        for der in &certificates {
            pem.push_str("-----BEGIN CERTIFICATE-----\n");
            let text = base64::engine::general_purpose::STANDARD.encode(der);
            for line in text.as_bytes().chunks(64) {
                // The text of base64 is ASCII, so each piece is text.
                let _ = writeln!(pem, "{}", String::from_utf8_lossy(line));
            }
            pem.push_str("-----END CERTIFICATE-----\n");
        }
        Ok(Self {
            pem,
            count: certificates.len(),
        })
    }

    /// The certificates as PEM text, as they are stored.
    pub fn pem(&self) -> &str {
        &self.pem
    }

    /// How many certificates the file holds.
    pub const fn count(&self) -> usize {
        self.count
    }
}

/// Whether a certificate is that of a certificate authority.
fn check_authority(der: &[u8]) -> Result<(), CertificateError> {
    let (rest, certificate) =
        X509Certificate::from_der(der).map_err(|_| CertificateError::Unreadable)?;
    if !rest.is_empty() {
        return Err(CertificateError::Unreadable);
    }
    let is_authority = certificate
        .basic_constraints()
        .map_err(|_| CertificateError::Unreadable)?
        .is_some_and(|constraints| constraints.value.ca);
    let may_sign = certificate
        .key_usage()
        .map_err(|_| CertificateError::Unreadable)?
        .is_none_or(|usage| usage.value.key_cert_sign());
    if is_authority && may_sign {
        Ok(())
    } else {
        Err(CertificateError::NotAnAuthority)
    }
}
