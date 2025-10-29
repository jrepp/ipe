use crate::bytecode::{CompiledPolicy, Instruction};
use crate::rar::EvaluationContext;
use crate::{Error, Result};
use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};
use parking_lot::RwLock;
use region::{protect, Protection};
use std::collections::HashMap;
use std::sync::Arc;

/// JIT-compiled native code for a policy
pub struct JitCode {
    /// Function pointer to native code
    ptr: *const u8,
    /// Size of compiled code
    size: usize,
    /// Memory region (for cleanup)
    region: *mut u8,
}

unsafe impl Send for JitCode {}
unsafe impl Sync for JitCode {}

impl JitCode {
    /// Execute the JIT-compiled policy
    ///
    /// # Safety
    /// Caller must ensure the context pointer is valid
    pub unsafe fn execute(&self, ctx: *const EvaluationContext) -> bool {
        let func: extern "C" fn(*const EvaluationContext) -> u8 = std::mem::transmute(self.ptr);
        func(ctx) != 0
    }
}

impl Drop for JitCode {
    fn drop(&mut self) {
        // Note: region-allocated memory is automatically freed when the protection is dropped
        // The `region` crate doesn't provide an explicit `free` function
    }
}

/// JIT compiler for policies
pub struct JitCompiler {
    /// Cranelift JIT module
    module: JITModule,
    /// Builder context (reused)
    builder_ctx: FunctionBuilderContext,
    /// Compiled functions cache
    cache: Arc<RwLock<HashMap<String, Arc<JitCode>>>>,
}

impl JitCompiler {
    pub fn new() -> Result<Self> {
        let mut flag_builder = settings::builder();
        flag_builder
            .set("opt_level", "speed")
            .map_err(|e| Error::JitError(format!("Failed to set optimization level: {}", e)))?;

        flag_builder
            .set("is_pic", "false")
            .map_err(|e| Error::JitError(format!("Failed to disable PIC: {}", e)))?;

        let isa_builder = cranelift_native::builder()
            .map_err(|e| Error::JitError(format!("Failed to get native ISA: {}", e)))?;

        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .map_err(|e| Error::JitError(format!("Failed to create ISA: {}", e)))?;

        let builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());

        let module = JITModule::new(builder);

        Ok(Self {
            module,
            builder_ctx: FunctionBuilderContext::new(),
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Compile a policy to native code
    pub fn compile(&mut self, policy: &CompiledPolicy, name: &str) -> Result<Arc<JitCode>> {
        // Check cache
        {
            let cache = self.cache.read();
            if let Some(code) = cache.get(name) {
                return Ok(Arc::clone(code));
            }
        }

        // Create function signature
        // extern "C" fn(*const EvaluationContext) -> u8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64)); // ctx pointer
        sig.returns.push(AbiParam::new(types::I8)); // bool result

        // Declare function
        let id = self
            .module
            .declare_function(name, Linkage::Export, &sig)
            .map_err(|e| Error::JitError(format!("Failed to declare function: {}", e)))?;

        // Create function context
        let mut ctx = self.module.make_context();
        ctx.func.signature = sig;

        // Build function body
        {
            let mut builder = FunctionBuilder::new(&mut ctx.func, &mut self.builder_ctx);

            // Entry block with ctx parameter
            let entry_block = builder.create_block();
            builder.append_block_params_for_function_params(entry_block);
            builder.switch_to_block(entry_block);
            builder.seal_block(entry_block);

            let ctx_ptr = builder.block_params(entry_block)[0];

            // Translate bytecode to IR
            Self::translate_bytecode(&mut builder, policy, ctx_ptr)?;

            builder.finalize();
        }

        // Define and compile
        self.module
            .define_function(id, &mut ctx)
            .map_err(|e| Error::JitError(format!("Failed to define function: {}", e)))?;

        self.module
            .finalize_definitions()
            .map_err(|e| Error::JitError(format!("Failed to finalize: {}", e)))?;

        // Get function pointer
        let code_ptr = self.module.get_finalized_function(id);

        // Make memory executable
        let jit_code = Arc::new(JitCode {
            ptr: code_ptr as *const u8,
            size: 4096, // Page size estimate
            region: code_ptr as *mut u8,
        });

        // Protect memory as executable
        unsafe {
            protect(jit_code.region, jit_code.size, Protection::READ_EXECUTE)
                .map_err(|e| Error::JitError(format!("Failed to protect memory: {}", e)))?;
        }

        // Cache result
        self.cache.write().insert(name.to_string(), Arc::clone(&jit_code));

        Ok(jit_code)
    }

    fn translate_bytecode(
        builder: &mut FunctionBuilder,
        policy: &CompiledPolicy,
        ctx_ptr: Value,
    ) -> Result<()> {
        // Stack for intermediate values
        let mut value_stack: Vec<Value> = Vec::new();

        // Block map for jumps
        let mut block_map: HashMap<usize, Block> = HashMap::new();

        // Create blocks for jump targets
        for (idx, instr) in policy.code.iter().enumerate() {
            match instr {
                Instruction::Jump { offset } | Instruction::JumpIfFalse { offset } => {
                    let target = (idx as i16 + offset) as usize;
                    if !block_map.contains_key(&target) {
                        block_map.insert(target, builder.create_block());
                    }
                },
                _ => {},
            }
        }

        // Translate instructions
        for (idx, instr) in policy.code.iter().enumerate() {
            // If this is a jump target, seal previous block and switch
            if let Some(&block) = block_map.get(&idx) {
                builder.seal_block(block);
                builder.switch_to_block(block);
            }

            match instr {
                Instruction::LoadField { offset } => {
                    // Load field from context: *(ctx + offset)
                    let field_addr = builder.ins().iadd_imm(ctx_ptr, *offset as i64);
                    let value = builder.ins().load(types::I64, MemFlags::trusted(), field_addr, 0);
                    value_stack.push(value);
                },

                Instruction::LoadConst { idx } => {
                    // Load constant from constant pool
                    let constant = &policy.constants[*idx as usize];
                    let value = match constant {
                        crate::bytecode::Value::Int(i) => builder.ins().iconst(types::I64, *i),
                        crate::bytecode::Value::Bool(b) => {
                            builder.ins().iconst(types::I8, if *b { 1 } else { 0 })
                        },
                        crate::bytecode::Value::String(_) => {
                            // For strings, we'd need to store them in data section
                            // For now, just use a placeholder
                            builder.ins().iconst(types::I64, 0)
                        },
                    };
                    value_stack.push(value);
                },

                Instruction::Compare { op } => {
                    let b = value_stack
                        .pop()
                        .ok_or_else(|| Error::JitError("Stack underflow in Compare".to_string()))?;
                    let a = value_stack
                        .pop()
                        .ok_or_else(|| Error::JitError("Stack underflow in Compare".to_string()))?;

                    let result = match op {
                        crate::bytecode::CompOp::Eq => builder.ins().icmp(IntCC::Equal, a, b),
                        crate::bytecode::CompOp::Neq => builder.ins().icmp(IntCC::NotEqual, a, b),
                        crate::bytecode::CompOp::Lt => {
                            builder.ins().icmp(IntCC::SignedLessThan, a, b)
                        },
                        crate::bytecode::CompOp::Lte => {
                            builder.ins().icmp(IntCC::SignedLessThanOrEqual, a, b)
                        },
                        crate::bytecode::CompOp::Gt => {
                            builder.ins().icmp(IntCC::SignedGreaterThan, a, b)
                        },
                        crate::bytecode::CompOp::Gte => {
                            builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, a, b)
                        },
                    };

                    value_stack.push(result);
                },

                Instruction::Jump { offset } => {
                    let target = (idx as i16 + offset) as usize;
                    let target_block = block_map[&target];
                    builder.ins().jump(target_block, &[]);
                },

                Instruction::JumpIfFalse { offset } => {
                    let cond = value_stack.pop().ok_or_else(|| {
                        Error::JitError("Stack underflow in JumpIfFalse".to_string())
                    })?;

                    let target = (idx as i16 + offset) as usize;
                    let target_block = block_map[&target];

                    // Create fallthrough block
                    let fallthrough = builder.create_block();

                    builder.ins().brif(cond, fallthrough, &[], target_block, &[]);
                    builder.seal_block(fallthrough);
                    builder.switch_to_block(fallthrough);
                },

                Instruction::And => {
                    let b = value_stack
                        .pop()
                        .ok_or_else(|| Error::JitError("Stack underflow in And".to_string()))?;
                    let a = value_stack
                        .pop()
                        .ok_or_else(|| Error::JitError("Stack underflow in And".to_string()))?;
                    let result = builder.ins().band(a, b);
                    value_stack.push(result);
                },

                Instruction::Or => {
                    let b = value_stack
                        .pop()
                        .ok_or_else(|| Error::JitError("Stack underflow in Or".to_string()))?;
                    let a = value_stack
                        .pop()
                        .ok_or_else(|| Error::JitError("Stack underflow in Or".to_string()))?;
                    let result = builder.ins().bor(a, b);
                    value_stack.push(result);
                },

                Instruction::Not => {
                    let a = value_stack
                        .pop()
                        .ok_or_else(|| Error::JitError("Stack underflow in Not".to_string()))?;
                    let result = builder.ins().bnot(a);
                    value_stack.push(result);
                },

                Instruction::Call { func: _, argc: _ } => {
                    // Built-in function calls
                    // For now, just push a dummy result
                    let result = builder.ins().iconst(types::I64, 0);
                    value_stack.push(result);
                },

                Instruction::Return { value } => {
                    let ret_val = if *value {
                        builder.ins().iconst(types::I8, 1)
                    } else {
                        builder.ins().iconst(types::I8, 0)
                    };
                    builder.ins().return_(&[ret_val]);
                },
            }
        }

        // Note: Return instructions are handled in bytecode translation
        // Each bytecode sequence should end with a Return instruction
        Ok(())
    }
}

impl Default for JitCompiler {
    fn default() -> Self {
        Self::new().expect("Failed to create JIT compiler")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::{CompOp, CompiledPolicy, Instruction, PolicyHeader, Value};

    #[test]
    #[cfg_attr(miri, ignore = "JIT compilation requires pointer operations not supported by Miri")]
    fn test_simple_jit_compilation() {
        let mut compiler = JitCompiler::new().unwrap();

        // Simple policy: always return true
        let policy = CompiledPolicy {
            header: PolicyHeader {
                magic: *b"IPE\0",
                version: 1,
                policy_id: 0,
                code_size: 1,
                const_size: 0,
            },
            code: vec![Instruction::Return { value: true }],
            constants: vec![],
        };

        let jit_code = compiler.compile(&policy, "test_policy").unwrap();

        // Test execution
        let ctx = EvaluationContext::default();
        let result = unsafe { jit_code.execute(&ctx as *const _) };
        assert!(result);
    }
}
