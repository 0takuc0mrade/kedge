// Re-export the generated guest method constants.
// After `risc0_build::embed_methods()` runs in build.rs,
// this include! pulls in the generated code containing:
//   - CLAIM_EVALUATOR_ELF: &[u8]  (the guest binary)
//   - CLAIM_EVALUATOR_ID: [u32; 8] (the image ID)
include!(concat!(env!("OUT_DIR"), "/methods.rs"));
