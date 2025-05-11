//! Loader for SPIR-V bytecode.
//!
//! # References
//!
//! - <https://registry.khronos.org/SPIR-V/specs/unified1/SPIRV.html>

mod ops;

use std::borrow::Cow;
use std::fmt::{self, Display, Formatter};
use std::num::NonZeroU32;

use bitflags::Flags;
use hashbrown::{HashMap, HashSet};
use ops::{
    Decoration, Id, Instruction, OpCapability, OpConstant, OpConstantComposite, OpConstantFalse,
    OpConstantNull, OpConstantSampler, OpConstantTrue, OpDecorate, OpEntryPoint, OpFunction,
    OpFunctionEnd, OpFunctionParameter, OpMemberDecorate, OpMemoryModel, OpSpecConstant,
    OpSpecConstantComposite, OpSpecConstantFalse, OpSpecConstantOp, OpSpecConstantTrue,
    OpTypeArray, OpTypeBool, OpTypeFloat, OpTypeFunction, OpTypeImage, OpTypeInt, OpTypeMatrix,
    OpTypePointer, OpTypeRuntimeArray, OpTypeSampledImage, OpTypeSampler, OpTypeStruct,
    OpTypeVector, OpTypeVoid, OpVariable, Parse,
};
use spirv::{Capability, ExecutionModel, StorageClass, MAGIC_NUMBER};
use thiserror::Error;

use crate::backend::{DescriptorType, ShaderStage};
use crate::shader::ShaderAccess;

use super::{BindingLocation, Options, ShaderBinding};

#[derive(Debug, Error)]
#[error(transparent)]
pub struct Error(#[from] ErrorImpl);

#[derive(Debug, Error)]
enum ErrorImpl {
    #[error("incomplete word: the stream is not a multiple of 4 bytes")]
    IncompleteWord,
    #[error("incomplete header")]
    IncompleteHeader,
    #[error("bad magic: {0}")]
    BadMagic(u32),
    #[error("invalid instruction: {0}")]
    InvalidArgumentCount(InvalidArgumentCount),
    #[error("unknown type: {0}")]
    UnknownType(Id),
    #[error("invalid type value: {found:?}, expected: {expected:?}")]
    InvalidTypeValue {
        found: OpTypeKind,
        expected: OpTypeKind,
    },
    #[error("unknown value {1} for enum {0}")]
    UnknownEnumValue(&'static str, u32),
    #[error(transparent)]
    InvalidString(std::string::FromUtf8Error),
    #[error("unknown opcode: {0}")]
    UnknownOpcode(u32),
    #[error("cannot reopen a new block until the previous block was sealed")]
    ReopenBlock,
    #[error("unknown entry point {name} with stage {stage:?}")]
    UnknownEntryPoint { name: String, stage: ShaderStage },
    #[error("no binding at {0:?}")]
    NoBinding(BindingLocation),
    #[error("invalid type to specialize: {0}")]
    InvalidTypeToSpecialize(OpTypeKind),
    #[error("unexpected instruction: {0:?}")]
    UnexpectedInstruction(Instruction),
    #[error("unknown storage class: {0:?}")]
    UnkownStorageClass(StorageClass),
    #[error("unknown decoration: {0:?}")]
    UnknownDecoration(spirv::Decoration),
}

#[derive(Copy, Clone, Debug, Error)]
pub struct InvalidArgumentCount {
    op: &'static str,
    required: usize,
    found: usize,
    variable: bool,
}

impl Display for InvalidArgumentCount {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.variable {
            write!(
                f,
                "{} requires at least {} arguments (found {})",
                self.op, self.required, self.found
            )
        } else {
            write!(
                f,
                "{} requires {} argument (found {})",
                self.op, self.required, self.found
            )
        }
    }
}

#[derive(Clone, Debug)]
pub struct Module {
    module: SpirvModule,
    bindings: HashMap<Id, ShaderBinding>,
}

impl Module {
    pub fn new(bytes: &[u8]) -> Result<Self, Error> {
        let module = SpirvModule::read(bytes)?;

        let mut bindings = HashMap::new();
        for global in module.globals.values() {
            let Some(ty) = module.types.get(&global.result_type) else {
                return Err(ErrorImpl::UnknownType(global.result_type).into());
            };

            // Global is a `OpVariable` and `Result Type` msut be an `OpTypePointer`.
            let ptr = match ty {
                OpType::Pointer(ptr) => ptr,
                _ => {
                    return Err(ErrorImpl::InvalidTypeValue {
                        found: ty.kind(),
                        expected: OpTypeKind::Pointer,
                    }
                    .into());
                }
            };

            let kind;
            let mut count = Some(NonZeroU32::MIN);
            match global.storage_class {
                // Input and Output classes are vertex attributes.
                StorageClass::Input | StorageClass::Output => continue,
                StorageClass::StorageBuffer => {
                    kind = DescriptorType::Storage;
                }
                StorageClass::UniformConstant => {
                    let Some(ty) = module.types.get(&ptr.type_) else {
                        return Err(ErrorImpl::UnknownType(ptr.type_).into());
                    };

                    let mut next_ty = ty;
                    loop {
                        kind = match next_ty {
                            OpType::Image(_) => DescriptorType::Texture,
                            OpType::Sampler => DescriptorType::Sampler,
                            OpType::Array(ty) => {
                                if let Some(count) = &mut count {
                                    *count = count
                                        .checked_mul(NonZeroU32::new(ty.length).unwrap())
                                        .unwrap();
                                }

                                let Some(ty) = module.types.get(&ty.element_type) else {
                                    return Err(ErrorImpl::UnknownType(ty.element_type).into());
                                };

                                next_ty = ty;
                                continue;
                            }
                            OpType::RuntimeArray(ty) => {
                                count = None;

                                let Some(ty) = module.types.get(&ty.element_type) else {
                                    return Err(ErrorImpl::UnknownType(ty.element_type).into());
                                };

                                next_ty = ty;
                                continue;
                            }
                            _ => DescriptorType::Uniform,
                        };

                        break;
                    }
                }
                _ => return Err(ErrorImpl::UnkownStorageClass(global.storage_class).into()),
            }

            bindings.insert(
                global.result,
                ShaderBinding {
                    group: 0,
                    binding: 0,
                    kind,
                    access: ShaderAccess::empty(),
                    count,
                },
            );
        }

        for (target, decorations) in &module.decorations {
            for decoration in decorations {
                match decoration {
                    Decoration::Binding(id) => {
                        if let Some(binding) = bindings.get_mut(target) {
                            binding.binding = *id;
                        }
                    }
                    Decoration::DescriptorSet(id) => {
                        if let Some(binding) = bindings.get_mut(target) {
                            binding.group = *id;
                        }
                    }
                    _ => (),
                }
            }
        }

        for entry_point in module.entry_points.values() {
            let variables = module.compute_global_accesses(entry_point.entry_point);
            for (var_id, access) in variables {
                if let Some(binding) = bindings.get_mut(&var_id) {
                    binding.access |= access;
                }
            }
        }

        Ok(Self { module, bindings })
    }

    pub fn bindings(&self) -> Vec<ShaderBinding> {
        self.bindings.values().copied().collect()
    }

    pub fn instantiate(&self, options: &Options<'_>) -> Result<Instance, Error> {
        let Some(entry_point) = self.module.entry_points.get(options.entry_point) else {
            return Err(ErrorImpl::UnknownEntryPoint {
                name: options.entry_point.to_owned(),
                stage: options.stage,
            }
            .into());
        };

        match (options.stage, entry_point.execution_model) {
            (ShaderStage::Vertex, ExecutionModel::Vertex) => (),
            (ShaderStage::Fragment, ExecutionModel::Fragment) => (),
            _ => {
                return Err(ErrorImpl::UnknownEntryPoint {
                    name: options.entry_point.to_owned(),
                    stage: options.stage,
                }
                .into());
            }
        }

        let mut module = self.module.clone();

        let mut bindings = self.bindings.clone();
        let variables = module.compute_global_accesses(entry_point.entry_point);
        for (id, binding) in &mut bindings {
            binding.access.clear();
            if let Some(access) = variables.get(id) {
                binding.access = *access;
            }
        }

        let len_type_id = module.header.allocate_id();
        module.types.insert(
            len_type_id,
            OpType::Int(OpTypeInt {
                result: len_type_id,
                width: 32,
                is_signed: false,
            }),
        );

        for (location, info) in &options.bindings {
            let Some((var_id, binding)) = bindings.iter_mut().find(|(_, binding)| {
                binding.binding == location.binding && binding.group == location.group
            }) else {
                return Err(ErrorImpl::NoBinding(*location).into());
            };

            let variable = module.globals.get(var_id).unwrap();

            let ptr_type = match module.types.get(&variable.result_type).unwrap() {
                OpType::Pointer(v) => *v,
                // We have already checked that all `OpVariable` instructions are
                // well formed and point have a `OpTypePointer` type.
                _ => unreachable!(),
            };
            let array_type = match module.types.get(&ptr_type.type_).unwrap() {
                OpType::RuntimeArray(v) => *v,
                ty => return Err(ErrorImpl::InvalidTypeToSpecialize(ty.kind()).into()),
            };

            // Create a new `OpConstant` and `OpArray` with the new
            // array length. We avoid modifying the original `OpTypeRuntimeArray`
            // and `OpTypePointer` since they may still be used by other variables.
            let new_array_len_id = module.header.allocate_id();
            module.constants.insert(
                new_array_len_id,
                Constant::Constant(OpConstant {
                    result_type: len_type_id,
                    result: new_array_len_id,
                    value: vec![info.count.get()],
                }),
            );

            let new_array_type_id = module.header.allocate_id();
            module.types.insert(
                new_array_type_id,
                OpType::Array(OpTypeArray {
                    result: new_array_type_id,
                    length: new_array_len_id,
                    element_type: array_type.element_type,
                }),
            );

            let new_ptr_type_id = module.header.allocate_id();
            module.types.insert(
                new_ptr_type_id,
                OpType::Pointer(OpTypePointer {
                    result: new_ptr_type_id,
                    storage_class: ptr_type.storage_class,
                    type_: new_array_type_id,
                }),
            );

            // Update the original `OpVariable` to point to the new `OpTypePointer`
            // which now points to `OpTypeArray`.
            let variable = module.globals.get_mut(var_id).unwrap();
            variable.result_type = new_ptr_type_id;

            binding.count = Some(info.count);
        }

        Ok(Instance {
            data: module,
            bindings: bindings.into_values().collect(),
        })
    }
}

#[derive(Clone, Debug)]
struct SpirvModule {
    header: Header,
    capabilities: HashSet<Capability>,
    extensions: Vec<Instruction>,
    memory_model: OpMemoryModel,
    entry_points: HashMap<String, OpEntryPoint>,
    execution_modes: Vec<Instruction>,
    debug: Vec<Instruction>,
    decorations: HashMap<Id, Vec<Decoration>>,
    member_decorations: HashMap<Id, Vec<OpMemberDecorate>>,
    types: HashMap<Id, OpType>,
    constants: HashMap<Id, Constant>,
    globals: HashMap<Id, OpVariable>,
    functions: HashMap<Id, Function>,
}

impl SpirvModule {
    fn read(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() % 4 != 0 {
            return Err(ErrorImpl::IncompleteWord.into());
        }

        // If the word is already aligned we can cast the
        // slice in place, otherwise we need to reallocate
        // and copy all words.
        let words = match bytemuck::try_cast_slice(bytes) {
            Ok(words) => Cow::Borrowed(words),
            Err(_) => Cow::Owned(
                bytes
                    .chunks(4)
                    .map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap()))
                    .collect(),
            ),
        };

        let endian = match words.first().copied() {
            Some(v) if v == MAGIC_NUMBER => Endianess::NATIVE,
            Some(v) if v == MAGIC_NUMBER.reverse_bits() => Endianess::NATIVE.reverse(),
            Some(v) => return Err(ErrorImpl::BadMagic(v).into()),
            None => return Err(ErrorImpl::IncompleteHeader.into()),
        };

        let mut reader = WordReader {
            words: &words,
            endian,
        };

        let header = Header::read(&mut reader)?;

        let mut capabilities = HashSet::new();
        let mut extensions = Vec::new();
        let mut memory_model = None;
        let mut entry_points = HashMap::new();
        let mut execution_modes = Vec::new();
        let mut debug = Vec::new();
        let mut decorations = HashMap::<_, Vec<_>>::new();
        let mut member_decorations = HashMap::<_, Vec<_>>::new();
        let mut types = HashMap::new();
        let mut constants = HashMap::new();
        let mut globals = HashMap::new();
        let mut functions = HashMap::new();

        while reader.len() != 0 {
            let instruction = Instruction::read(&mut reader)?;
            match instruction {
                Instruction::Capability(ins) => {
                    capabilities.insert(ins.capability);
                }
                Instruction::Extension(ins) => {
                    extensions.push(Instruction::Extension(ins));
                }
                Instruction::ExtInstImport(ins) => {
                    extensions.push(Instruction::ExtInstImport(ins));
                }
                Instruction::MemoryModel(ins) => {
                    assert!(memory_model.is_none());
                    memory_model = Some(ins);
                }
                Instruction::EntryPoint(ins) => {
                    entry_points.insert(ins.name.clone(), ins);
                }
                Instruction::ExecutionMode(ins) => {
                    execution_modes.push(Instruction::ExecutionMode(ins));
                }
                Instruction::ExecutionModeId(ins) => {
                    execution_modes.push(Instruction::ExecutionModeId(ins));
                }
                Instruction::String(ins) => {
                    debug.push(Instruction::String(ins));
                }
                Instruction::SourceExtension(ins) => {
                    debug.push(Instruction::SourceExtension(ins));
                }
                Instruction::Source(ins) => {
                    debug.push(Instruction::Source(ins));
                }
                Instruction::SourceContinued(ins) => {
                    debug.push(Instruction::SourceContinued(ins));
                }
                Instruction::Name(ins) => {
                    debug.push(Instruction::Name(ins));
                }
                Instruction::MemberName(ins) => {
                    debug.push(Instruction::MemberName(ins));
                }
                Instruction::ModuleProcessed(ins) => {
                    debug.push(Instruction::ModuleProcessed(ins));
                }
                Instruction::Decorate(ins) => {
                    decorations
                        .entry(ins.target)
                        .or_default()
                        .push(ins.decoration);
                }
                Instruction::MemberDecorate(ins) => {
                    member_decorations
                        .entry(ins.structure_type)
                        .or_default()
                        .push(ins);
                }
                Instruction::TypeVoid(ins) => {
                    types.insert(ins.result, OpType::Void);
                }
                Instruction::TypeBool(ins) => {
                    types.insert(ins.result, OpType::Bool);
                }
                Instruction::TypeInt(ins) => {
                    types.insert(ins.result, OpType::Int(ins));
                }
                Instruction::TypeFloat(ins) => {
                    types.insert(ins.result, OpType::Float(ins));
                }
                Instruction::TypeVector(ins) => {
                    types.insert(ins.result, OpType::Vector(ins));
                }
                Instruction::TypeMatrix(ins) => {
                    types.insert(ins.result, OpType::Matrix(ins));
                }
                Instruction::TypeImage(ins) => {
                    types.insert(ins.result, OpType::Image(ins));
                }
                Instruction::TypeSampler(ins) => {
                    types.insert(ins.result, OpType::Sampler);
                }
                Instruction::TypeSampledImage(ins) => {
                    types.insert(ins.result, OpType::SampledImage(ins));
                }
                Instruction::TypeArray(ins) => {
                    types.insert(ins.result, OpType::Array(ins));
                }
                Instruction::TypeRuntimeArray(ins) => {
                    types.insert(ins.result, OpType::RuntimeArray(ins));
                }
                Instruction::TypeStruct(ins) => {
                    types.insert(ins.result, OpType::Struct(ins));
                }
                Instruction::TypePointer(ins) => {
                    types.insert(ins.result, OpType::Pointer(ins));
                }
                Instruction::TypeFunction(ins) => {
                    types.insert(ins.result, OpType::Function(ins));
                }
                Instruction::ConstantTrue(ins) => {
                    constants.insert(ins.result, Constant::ConstantTrue(ins));
                }
                Instruction::ConstantFalse(ins) => {
                    constants.insert(ins.result, Constant::ConstantFalse(ins));
                }
                Instruction::Constant(ins) => {
                    constants.insert(ins.result, Constant::Constant(ins));
                }
                Instruction::ConstantComposite(ins) => {
                    constants.insert(ins.result, Constant::ConstantComposite(ins));
                }
                Instruction::ConstantSampler(ins) => {
                    constants.insert(ins.result, Constant::ConstantSampler(ins));
                }
                Instruction::ConstantNull(ins) => {
                    constants.insert(ins.result, Constant::ConstantNull(ins));
                }
                Instruction::SpecConstantTrue(ins) => {
                    constants.insert(ins.result, Constant::SpecConstantTrue(ins));
                }
                Instruction::SpecConstantFalse(ins) => {
                    constants.insert(ins.result, Constant::SpecConstantFalse(ins));
                }
                Instruction::SpecConstant(ins) => {
                    constants.insert(ins.result, Constant::SpecConstant(ins));
                }
                Instruction::SpecConstantComposite(ins) => {
                    constants.insert(ins.result, Constant::SpecConstantComposite(ins));
                }
                Instruction::SpecConstantOp(ins) => {
                    constants.insert(ins.result, Constant::SpecConstantOp(ins));
                }
                Instruction::Variable(ins) => {
                    globals.insert(ins.result, ins);
                }
                Instruction::Function(ins) => {
                    let f = Function::read(ins, &mut reader)?;
                    functions.insert(ins.result, f);
                }
                _ => return Err(ErrorImpl::UnexpectedInstruction(instruction).into()),
            }
        }

        Ok(Self {
            header,
            capabilities,
            extensions,
            memory_model: memory_model.unwrap(),
            entry_points,
            execution_modes,
            debug,
            decorations,
            member_decorations,
            types,
            constants,
            globals,
            functions,
        })
    }

    fn write(&self, writer: &mut Vec<u32>) {
        self.header.write(writer);

        for capability in &self.capabilities {
            Instruction::Capability(OpCapability {
                capability: *capability,
            })
            .write(writer);
        }

        for ins in &self.extensions {
            ins.write(writer);
        }

        Instruction::MemoryModel(self.memory_model).write(writer);

        for entry_point in self.entry_points.values() {
            Instruction::EntryPoint(entry_point.clone()).write(writer);
        }

        for mode in &self.execution_modes {
            mode.write(writer);
        }

        for ins in &self.debug {
            ins.write(writer);
        }

        for (target, decorations) in &self.decorations {
            for decoration in decorations {
                Instruction::Decorate(OpDecorate {
                    target: *target,
                    decoration: *decoration,
                })
                .write(writer);
            }
        }

        for (_, decorations) in &self.member_decorations {
            for ins in decorations {
                ins.write(writer);
            }
        }

        for (id, ty) in &self.types {
            let ins = match ty {
                OpType::Void => Instruction::TypeVoid(OpTypeVoid { result: *id }),
                OpType::Bool => Instruction::TypeBool(OpTypeBool { result: *id }),
                OpType::Int(ty) => Instruction::TypeInt(*ty),
                OpType::Float(ty) => Instruction::TypeFloat(*ty),
                OpType::Vector(ty) => Instruction::TypeVector(*ty),
                OpType::Matrix(ty) => Instruction::TypeMatrix(*ty),
                OpType::Image(ty) => Instruction::TypeImage(*ty),
                OpType::Sampler => Instruction::TypeSampler(OpTypeSampler { result: *id }),
                OpType::SampledImage(ty) => Instruction::TypeSampledImage(*ty),
                OpType::Array(ty) => Instruction::TypeArray(*ty),
                OpType::RuntimeArray(ty) => Instruction::TypeRuntimeArray(*ty),
                OpType::Struct(ty) => Instruction::TypeStruct(ty.clone()),
                OpType::Pointer(ty) => Instruction::TypePointer(*ty),
                OpType::Function(ty) => Instruction::TypeFunction(ty.clone()),
            };

            ins.write(writer);
        }

        for constant in self.constants.values() {
            let ins = match constant {
                Constant::ConstantTrue(v) => Instruction::ConstantTrue(*v),
                Constant::ConstantFalse(v) => Instruction::ConstantFalse(*v),
                Constant::Constant(v) => Instruction::Constant(v.clone()),
                Constant::ConstantComposite(v) => Instruction::ConstantComposite(v.clone()),
                Constant::ConstantSampler(v) => Instruction::ConstantSampler(*v),
                Constant::ConstantNull(v) => Instruction::ConstantNull(*v),
                Constant::SpecConstantTrue(v) => Instruction::SpecConstantTrue(*v),
                Constant::SpecConstantFalse(v) => Instruction::SpecConstantFalse(*v),
                Constant::SpecConstant(v) => Instruction::SpecConstant(*v),
                Constant::SpecConstantComposite(v) => Instruction::SpecConstantComposite(v.clone()),
                Constant::SpecConstantOp(v) => Instruction::SpecConstantOp(v.clone()),
            };

            ins.write(writer);
        }

        for function in self.functions.values() {
            function.write(writer);
        }
    }

    fn compute_global_accesses(&self, func_id: Id) -> HashMap<Id, ShaderAccess> {
        let mut variables: HashMap<_, _> = self
            .globals
            .keys()
            .map(|id| (*id, ShaderAccess::empty()))
            .collect();

        let mut proxies = HashMap::new();

        let mut queue = vec![func_id];

        while let Some(id) = queue.pop() {
            let func = &self.functions[&id];

            for instruction in func.blocks.iter().map(|b| &b.instructions).flatten() {
                match instruction {
                    Instruction::Load(ins) => {
                        if let Some(access) = variables.get_mut(&ins.pointer) {
                            *access |= ShaderAccess::READ;
                        }
                    }
                    Instruction::Store(ins) => {
                        if let Some(access) = variables.get_mut(&ins.pointer) {
                            *access |= ShaderAccess::WRITE;
                        }
                    }
                    Instruction::CopyMemory(ins) => {
                        if let Some(access) = variables.get_mut(&ins.source) {
                            *access |= ShaderAccess::READ;
                        }

                        if let Some(access) = variables.get_mut(&ins.target) {
                            *access |= ShaderAccess::WRITE;
                        }
                    }
                    Instruction::AccessChain(ins) => {
                        // `OpAccessChain` creates a pointer to the element `base`
                        // with the given `indices`.
                        // `OpAccessChain` does not directly access the `base` element
                        // but we must now track the returned result for operations.
                        let proxy_id = proxies.get(&ins.base).copied().unwrap_or(ins.base);
                        variables.insert(ins.result, ShaderAccess::empty());
                        proxies.insert(ins.result, proxy_id);
                    }
                    Instruction::InBoundsAccessChain(ins) => {
                        let proxy_id = proxies.get(&ins.base).copied().unwrap_or(ins.base);
                        variables.insert(ins.result, ShaderAccess::empty());
                        proxies.insert(ins.result, proxy_id);
                    }
                    Instruction::FunctionCall(ins) => {
                        queue.push(ins.function);
                    }
                    _ => (),
                }
            }
        }

        for (proxy_id, global_id) in proxies {
            let proxy_access = variables[&proxy_id];

            if let Some(accesses) = variables.get_mut(&global_id) {
                *accesses |= proxy_access
            }
        }

        // Remove all locally tracked variables.
        variables.retain(|id, _| self.globals.contains_key(id));

        variables
    }
}

#[derive(Copy, Clone, Debug)]
struct Header {
    major_version: u8,
    minor_version: u8,
    generator_magic: u32,
    bound: u32,
}

impl Header {
    fn read(reader: &mut WordReader<'_>) -> Result<Self, Error> {
        // Magic
        reader.next().ok_or(Error(ErrorImpl::IncompleteHeader))?;

        // Version
        let version = reader.next().ok_or(Error(ErrorImpl::IncompleteHeader))?;
        let major_version = ((version & 0x00FF_0000) >> 16) as u8;
        let minor_version = ((version & 0x0000_FF00) >> 8) as u8;

        let generator_magic = reader.next().ok_or(Error(ErrorImpl::IncompleteHeader))?;
        let bound = reader.next().ok_or(Error(ErrorImpl::IncompleteHeader))?;

        // Reserved
        reader.next().ok_or(Error(ErrorImpl::IncompleteHeader))?;

        Ok(Self {
            major_version,
            minor_version,
            generator_magic,
            bound,
        })
    }

    fn write(&self, writer: &mut Vec<u32>) {
        writer.push(MAGIC_NUMBER);

        let version =
            (u32::from(self.major_version) << 16) | (u32::from(self.minor_version as u32) << 8);
        writer.push(version);

        writer.push(self.generator_magic);
        writer.push(self.bound);
    }

    fn allocate_id(&mut self) -> Id {
        let id = self.bound;
        self.bound += 1;
        id
    }
}

#[derive(Clone, Debug)]
struct Function {
    function: OpFunction,
    parameters: Vec<OpFunctionParameter>,
    blocks: Vec<Block>,
}

impl Function {
    fn read(function: OpFunction, reader: &mut WordReader<'_>) -> Result<Self, Error> {
        // Whether the instruction is a "Function Termination Instruction".
        // https://registry.khronos.org/SPIR-V/specs/unified1/SPIRV.html#FunctionTermination
        let is_function_termination = |instruction: &Instruction| {
            matches!(
                instruction,
                Instruction::Return(_)
                    | Instruction::ReturnValue(_)
                    | Instruction::Kill(_)
                    | Instruction::Unreachable(_)
                    | Instruction::TerminateInvocation(_)
            )
        };

        let is_branch_instruction = |instruction: &Instruction| {
            matches!(
                instruction,
                Instruction::Branch(_) | Instruction::BranchConditional(_) | Instruction::Switch(_)
            )
        };

        // Whether the instruction is a "Block Termination Instruction"
        // https://registry.khronos.org/SPIR-V/specs/unified1/SPIRV.html#Termination
        let is_block_termination = |instruction: &Instruction| {
            is_branch_instruction(instruction) || is_function_termination(instruction)
        };

        let mut parameters = Vec::new();
        let mut blocks = Vec::new();

        let mut current_block = Vec::new();

        while reader.len() != 0 {
            let instruction = Instruction::read(reader)?;

            match instruction {
                Instruction::FunctionParameter(ins) => {
                    parameters.push(ins);
                }
                // Beginning of a block.
                Instruction::Label(_) => {
                    // The previous block must have been sealed
                    // before we can open a new block.
                    if !current_block.is_empty() {
                        return Err(ErrorImpl::ReopenBlock.into());
                    }

                    current_block.push(instruction);
                }
                Instruction::FunctionEnd(_) => {
                    break;
                }
                _ => {
                    current_block.push(instruction.clone());

                    if is_block_termination(&instruction) {
                        blocks.push(Block {
                            instructions: core::mem::take(&mut current_block),
                        });
                    }
                }
            }
        }

        Ok(Self {
            function,
            parameters,
            blocks,
        })
    }

    fn write(&self, writer: &mut Vec<u32>) {
        Instruction::Function(self.function).write(writer);
        for param in &self.parameters {
            Instruction::FunctionParameter(*param).write(writer);
        }

        for block in &self.blocks {
            for instruction in &block.instructions {
                instruction.write(writer);
            }
        }

        Instruction::FunctionEnd(OpFunctionEnd).write(writer);
    }
}

#[derive(Clone, Debug)]
struct Block {
    instructions: Vec<Instruction>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Endianess {
    Little,
    Big,
}

impl Endianess {
    /// Reverses the current endianess.
    const fn reverse(self) -> Self {
        match self {
            Self::Little => Self::Big,
            Self::Big => Self::Little,
        }
    }

    /// The native endianess.
    const NATIVE: Self = if cfg!(target_endian = "little") {
        Self::Little
    } else {
        Self::Big
    };
}

#[derive(Clone, Debug)]
pub struct Instance {
    data: SpirvModule,
    bindings: Vec<ShaderBinding>,
}

impl Instance {
    pub fn bindings(&self) -> &[ShaderBinding] {
        &self.bindings
    }

    pub fn to_spirv(&self) -> Vec<u32> {
        let mut words = Vec::new();
        self.data.write(&mut words);
        words
    }
}

#[derive(Clone, Debug)]
enum OpType {
    Void,
    Bool,
    Int(OpTypeInt),
    Float(OpTypeFloat),
    Vector(OpTypeVector),
    Matrix(OpTypeMatrix),
    Image(OpTypeImage),
    Sampler,
    SampledImage(OpTypeSampledImage),
    Array(OpTypeArray),
    RuntimeArray(OpTypeRuntimeArray),
    Struct(OpTypeStruct),
    Pointer(OpTypePointer),
    Function(OpTypeFunction),
}

impl OpType {
    fn kind(&self) -> OpTypeKind {
        match self {
            Self::Void => OpTypeKind::Void,
            Self::Bool => OpTypeKind::Bool,
            Self::Int(_) => OpTypeKind::Int,
            Self::Float(_) => OpTypeKind::Float,
            Self::Vector(_) => OpTypeKind::Vector,
            Self::Matrix(_) => OpTypeKind::Matrix,
            Self::Image(_) => OpTypeKind::Image,
            Self::Sampler => OpTypeKind::Sampler,
            Self::SampledImage(_) => OpTypeKind::SampledImage,
            Self::Array(_) => OpTypeKind::Array,
            Self::RuntimeArray(_) => OpTypeKind::RuntimeArray,
            Self::Struct(_) => OpTypeKind::Struct,
            Self::Pointer(_) => OpTypeKind::Pointer,
            Self::Function(_) => OpTypeKind::Function,
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum OpTypeKind {
    Void,
    Bool,
    Int,
    Float,
    Vector,
    Matrix,
    Image,
    Sampler,
    SampledImage,
    Array,
    RuntimeArray,
    Struct,
    Pointer,
    Function,
}

impl Display for OpTypeKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Void => "OpTypeVoid",
            Self::Bool => "OpTypeBool",
            Self::Int => "OpTypeInt",
            Self::Float => "OpTypeFloat",
            Self::Vector => "OpTypeVector",
            Self::Matrix => "OpTypeMatrix",
            Self::Image => "OpTypeImage",
            Self::Sampler => "OpTypeSampler",
            Self::SampledImage => "OpTypeSampledImage",
            Self::Array => "OpTypeArray",
            Self::RuntimeArray => "OpTypeRuntimeArray",
            Self::Struct => "OpTypeStruct",
            Self::Pointer => "OpTypePointer",
            Self::Function => "OpTypeFunction",
        };

        write!(f, "{}", s)
    }
}

#[derive(Clone, Debug)]
enum Constant {
    ConstantTrue(OpConstantTrue),
    ConstantFalse(OpConstantFalse),
    Constant(OpConstant),
    ConstantComposite(OpConstantComposite),
    ConstantSampler(OpConstantSampler),
    ConstantNull(OpConstantNull),
    SpecConstantTrue(OpSpecConstantTrue),
    SpecConstantFalse(OpSpecConstantFalse),
    SpecConstant(OpSpecConstant),
    SpecConstantComposite(OpSpecConstantComposite),
    SpecConstantOp(OpSpecConstantOp),
}

#[derive(Clone, Debug)]
struct WordReader<'a> {
    words: &'a [u32],
    endian: Endianess,
}

impl<'a> Iterator for WordReader<'a> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        let (word, rem) = self.words.split_first()?;
        self.words = rem;

        match (self.endian, Endianess::NATIVE) {
            (Endianess::Little, Endianess::Little) => Some(*word),
            (Endianess::Little, Endianess::Big) => Some(word.reverse_bits()),
            (Endianess::Big, Endianess::Little) => Some(word.reverse_bits()),
            (Endianess::Big, Endianess::Big) => Some(*word),
        }
    }
}

impl<'a> ExactSizeIterator for WordReader<'a> {
    fn len(&self) -> usize {
        self.words.len()
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use spirv_tools::assembler::{Assembler, AssemblerOptions};

    use crate::backend::ShaderStage;
    use crate::shader::{BindingInfo, BindingLocation, Options, ShaderAccess};

    use super::{Constant, Module, OpType, SpirvModule};

    fn assemble(text: &str) -> Vec<u8> {
        let assembler = spirv_tools::assembler::create(None);
        let options = AssemblerOptions {
            preserve_numeric_ids: true,
        };

        let binary = assembler.assemble(text, options).unwrap();
        binary.as_bytes().to_vec()
    }

    #[test]
    fn spirv_minimal() {
        let text = r#"
        OpMemoryModel Logical GLSL450
        "#;

        let bytes = assemble(text);
        SpirvModule::read(&bytes).unwrap();
    }

    #[test]
    fn compute_global_access_simple() {
        let text = r#"
        OpMemoryModel Logical GLSL450

        %ty_void    = OpTypeVoid
        %ty_f32     = OpTypeFloat 32
        %ty_ptr_f32 = OpTypePointer UniformConstant %ty_f32
        %ty_fn      = OpTypeFunction %ty_void

        %0 = OpVariable %ty_ptr_f32 UniformConstant
        %1 = OpVariable %ty_ptr_f32 UniformConstant
        %2 = OpVariable %ty_ptr_f32 UniformConstant

        %3 = OpFunction %ty_void None %ty_fn
        %4 = OpLabel
        %5 = OpLoad %ty_f32 %0
        %6 = OpLoad %ty_f32 %1
        %7 = OpIAdd %ty_f32 %5 %6
        OpStore %1 %7
        OpStore %2 %7
        OpReturn
        OpFunctionEnd
        "#;

        let bytes = assemble(text);
        let module = SpirvModule::read(&bytes).unwrap();
        let access = module.compute_global_accesses(3);
        assert_eq!(
            access,
            [
                (0, ShaderAccess::READ),
                (1, ShaderAccess::READ | ShaderAccess::WRITE),
                (2, ShaderAccess::WRITE),
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn compute_global_access_composite() {
        let text = r#"
        OpMemoryModel Logical GLSL450

        %ty_void      = OpTypeVoid
        %ty_u32       = OpTypeInt 32 0
        %ty_f32       = OpTypeFloat 32
        %ty_f32_ptr   = OpTypePointer UniformConstant %ty_f32
        %ty_fn        = OpTypeFunction %ty_void
        %ty_f32_3     = OpTypeArray %ty_f32 %const_3
        %ty_f32_3_ptr = OpTypePointer UniformConstant %ty_f32_3

        %const_1 = OpConstant %ty_u32 1
        %const_3 = OpConstant %ty_u32 3

        %0 = OpVariable %ty_f32_3 UniformConstant
        %1 = OpVariable %ty_f32_3 UniformConstant

        %2 = OpFunction %ty_void None %ty_fn
        %3 = OpLabel
        %4 = OpAccessChain %ty_f32_ptr %0 %const_1
        %5 = OpLoad %ty_f32 %4
        %6 = OpAccessChain %ty_f32_ptr %1 %const_1
        OpStore %6 %5
        OpReturn
        OpFunctionEnd
        "#;

        let bytes = assemble(text);
        let module = SpirvModule::read(&bytes).unwrap();
        let access = module.compute_global_accesses(2);
        assert_eq!(
            access,
            [(0, ShaderAccess::READ), (1, ShaderAccess::WRITE)]
                .into_iter()
                .collect()
        );
    }

    #[test]
    fn instantiate_specialize_runtime_array_length() {
        let text = r#"
        OpMemoryModel Logical GLSL450

        OpEntryPoint Vertex %1 "main"

        OpDecorate %0 DescriptorSet 0
        OpDecorate %1 Binding 0

        %ty_void = OpTypeVoid
        %ty_fn = OpTypeFunction %ty_void
        %ty_f32 = OpTypeFloat 32
        %ty_runtime_array_f32 = OpTypeRuntimeArray %ty_f32
        %ty_runtime_array_f32_ptr = OpTypePointer UniformConstant %ty_runtime_array_f32

        %0 = OpVariable %ty_runtime_array_f32_ptr UniformConstant

        %1 = OpFunction %ty_void None %ty_fn
        %2 = OpLabel
        OpReturn
        OpFunctionEnd
        "#;

        let bytes = assemble(text);
        let module = Module::new(&bytes).unwrap();
        let instance = module
            .instantiate(&Options {
                stage: ShaderStage::Vertex,
                entry_point: "main",
                bindings: [(
                    BindingLocation {
                        group: 0,
                        binding: 0,
                    },
                    BindingInfo {
                        count: NonZeroU32::new(420).unwrap(),
                    },
                )]
                .into_iter()
                .collect(),
            })
            .unwrap();

        let variable = instance.data.globals.get(&0).unwrap();
        let ptr_type = match instance.data.types.get(&variable.result_type).unwrap() {
            OpType::Pointer(v) => v,
            _ => unreachable!(),
        };

        let array_type = match instance.data.types.get(&ptr_type.type_).unwrap() {
            OpType::Array(v) => v,
            _ => unreachable!(),
        };

        let len = match instance.data.constants.get(&array_type.length).unwrap() {
            Constant::Constant(v) => v,
            _ => unreachable!(),
        };

        assert_eq!(len.value, vec![420]);
    }
}
