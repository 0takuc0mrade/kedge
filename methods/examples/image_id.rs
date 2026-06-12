use risc0_zkvm::sha::Digest;

fn main() {
    println!(
        "0x{}",
        hex::encode(Digest::from(kedge_methods::CLAIM_EVALUATOR_ID).as_bytes())
    );
}
