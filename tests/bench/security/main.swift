import CryptoKit
import Foundation

// Native/core round trips across generated Swift FFI, with no secret output.
var material = try probeRandomMaterial()
defer { material.resetBytes(in: 0..<material.count) }
precondition(material.count == 96)
var signing = Data(material[0..<32])
var agreement = Data(material[32..<64])
defer { signing.resetBytes(in: 0..<signing.count); agreement.resetBytes(in: 0..<agreement.count) }
let message = Data("MC005 synthetic host challenge".utf8)
let nativeSigning = try Curve25519.Signing.PrivateKey(rawRepresentation: signing)
let publicKey = try probeSigningPublic(secret: signing)
precondition(publicKey == nativeSigning.publicKey.rawRepresentation)
let signature = try probeSign(secret: signing, message: message)
precondition(nativeSigning.publicKey.isValidSignature(signature, for: message))
let nativeSignature = try nativeSigning.signature(for: message)
precondition(probeVerify(public: publicKey, message: message, signature: nativeSignature))
var bad = signature; bad[0] ^= 1
precondition(!probeVerify(public: publicKey, message: message, signature: bad))
let peer = Curve25519.KeyAgreement.PrivateKey()
let publicBytes = try probeAgreementPublic(secret: agreement)
let corePublic = try Curve25519.KeyAgreement.PublicKey(rawRepresentation: publicBytes)
let shared = try peer.sharedSecretFromKeyAgreement(with: corePublic)
var expected = shared.withUnsafeBytes { Data($0) }
defer { expected.resetBytes(in: 0..<expected.count) }
let matches = try probeAgreementMatches(secret: agreement, peer: peer.publicKey.rawRepresentation, expected: expected)
precondition(matches)
expected[0] ^= 1
let mismatch = try probeAgreementMatches(secret: agreement, peer: peer.publicKey.rawRepresentation, expected: expected)
precondition(!mismatch)
print("MC005 Swift FFI / CryptoKit Ed25519 and X25519 interoperability passed; hardware/storage untested")
