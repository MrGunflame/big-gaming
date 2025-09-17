use spirv::{
    self, AddressingModel, BuiltIn, Capability, Dim, ExecutionMode, ExecutionModel, FPFastMathMode,
    FPRoundingMode, FunctionControl, ImageFormat, LoopControl, MemoryModel, MemorySemantics,
    PackedVectorFormat, SamplerAddressingMode, SamplerFilterMode, SelectionControl, SourceLanguage,
    StorageClass,
};

use super::{Error, ErrorImpl, InvalidArgumentCount, WordReader};

#[derive(Clone, Debug)]
pub enum Instruction {
    Nop(OpNop),
    Undef(OpUndef),
    SizeOf(OpSizeOf),
    SourceContinued(OpSourceContinued),
    Source(OpSource),
    SourceExtension(OpSourceExtension),
    Name(OpName),
    MemberName(OpMemberName),
    String(OpString),
    Line(OpLine),
    NoLine(OpNoLine),
    ModuleProcessed(OpModuleProcessed),
    Decorate(OpDecorate),
    MemberDecorate(OpMemberDecorate),
    DecorationGroup(OpDecorationGroup),
    GroupDecorate(OpGroupDecorate),
    GroupMemberDecorate(OpGroupMemberDecorate),
    DecorateId(OpDecorateId),
    DecorateString(OpDecorateString),
    MemberDecorateString(OpMemberDecorateString),
    Extension(OpExtension),
    ExtInstImport(OpExtInstImport),
    ExtInst(OpExtInst),
    MemoryModel(OpMemoryModel),
    EntryPoint(OpEntryPoint),
    ExecutionMode(OpExecutionMode),
    Capability(OpCapability),
    ExecutionModeId(OpExecutionModeId),
    TypeVoid(OpTypeVoid),
    TypeBool(OpTypeBool),
    TypeInt(OpTypeInt),
    TypeFloat(OpTypeFloat),
    TypeVector(OpTypeVector),
    TypeMatrix(OpTypeMatrix),
    TypeImage(OpTypeImage),
    TypeSampler(OpTypeSampler),
    TypeSampledImage(OpTypeSampledImage),
    TypeArray(OpTypeArray),
    TypeRuntimeArray(OpTypeRuntimeArray),
    TypeStruct(OpTypeStruct),
    TypePointer(OpTypePointer),
    TypeFunction(OpTypeFunction),
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
    Variable(OpVariable),
    ImageTexelPointer(OpImageTexelPointer),
    Load(OpLoad),
    Store(OpStore),
    CopyMemory(OpCopyMemory),
    CopyMemorySized(OpCopyMemorySized),
    AccessChain(OpAccessChain),
    InBoundsAccessChain(OpInBoundsAccessChain),
    ArrayLength(OpArrayLength),
    Function(OpFunction),
    FunctionParameter(OpFunctionParameter),
    FunctionEnd(OpFunctionEnd),
    FunctionCall(OpFunctionCall),
    SampledImage(OpSampledImage),
    ImageSampleImplicitLod(OpImageSampleImplicitLod),
    ImageSampleExplicitLod(OpImageSampleExplicitLod),
    ImageSampleDrefImplicitLod(OpImageSampleDrefImplicitLod),
    ImageSampleDrefExplicitLod(OpImageSampleDrefExplicitLod),
    ImageSampleProjImplicitLod(OpImageSampleProjImplicitLod),
    ImageSampleProjExplicitLod(OpImageSampleProjExplicitLod),
    ImageSampleProjDrefImplicitLod(OpImageSampleProjDrefImplicitLod),
    ImageSampleProjDrefExplicitLod(OpImageSampleProjDrefExplicitLod),
    ImageFetch(OpImageFetch),
    ImageRead(OpImageRead),
    ImageWrite(OpImageWrite),
    ConvertFToU(OpConvertFToU),
    ConvertFToS(OpConvertFToS),
    ConvertSToF(OpConvertSToF),
    ConvertUToF(OpConvertUToF),
    UConvert(OpUConvert),
    SConvert(OpSConvert),
    FConvert(OpFConvert),
    QuantizeToF16(OpQuantizeToF16),
    SatConvertSToU(OpSatConvertSToU),
    SatConvertUToS(OpSatConvertUToS),
    Bitcast(OpBitcast),
    VectorExtractDynamic(OpVectorExtractDynamic),
    VectorInsertDynamic(OpVectorInsertDynamic),
    VectorShuffle(OpVectorShuffle),
    CompositeConstruct(OpCompositeConstruct),
    CompositeExtract(OpCompositeExtract),
    CompositeInsert(OpCompositeInsert),
    CopyObject(OpCopyObject),
    Transpose(OpTranspose),
    CopyLogical(OpCopyLogical),
    SNegate(OpSNegate),
    FNegate(OpFNegate),
    IAdd(OpIAdd),
    FAdd(OpFAdd),
    ISub(OpISub),
    FSub(OpFSub),
    IMul(OpIMul),
    FMul(OpFMul),
    UDiv(OpUDiv),
    SDiv(OpSDiv),
    FDiv(OpFDiv),
    UMod(OpUMod),
    SRem(OpSRem),
    SMod(OpSMod),
    FRem(OpFRem),
    FMod(OpFMod),
    VectorTimesScalar(OpVectorTimesScalar),
    MatrixTimesScalar(OpMatrixTimesScalar),
    VectorTimesMatrix(OpVectorTimesMatrix),
    MatrixTimesVector(OpMatrixTimesVector),
    MatrixTimesMatrix(OpMatrixTimesMatrix),
    OuterProduct(OpOuterProduct),
    Dot(OpDot),
    IAddCarry(OpIAddCarry),
    ISubBorrow(OpISubBorrow),
    UMulExtended(OpUMulExtended),
    SMulExtended(OpSMulExtended),
    SDot(OpSDot),
    UDot(OpUDot),
    SUDot(OpSUDot),
    SDotAccSat(OpSDotAccSat),
    UDotAccSat(OpUDotAccSat),
    SUDotAccSat(OpSUDotAccSat),
    ShiftRightLogical(OpShiftRightLogical),
    ShiftRightArithmetic(OpShiftRightArithmetic),
    ShiftLeftLogical(OpShiftLeftLogical),
    BitwiseOr(OpBitwiseOr),
    BitwiseXor(OpBitwiseXor),
    BitwiseAnd(OpBitwiseAnd),
    Not(OpNot),
    BitFieldInsert(OpBitFieldInsert),
    BitFieldSExtract(OpBitFieldSExtract),
    BitFieldUExtract(OpBitFieldUExtract),
    BitReverse(OpBitReverse),
    BitCount(OpBitCount),
    Any(OpAny),
    All(OpAll),
    IsNan(OpIsNan),
    IsInf(OpIsInf),
    IsFinite(OpIsFinite),
    IsNormal(OpIsNormal),
    SignBitSet(OpSignBitSet),
    LessOrGreater(OpLessOrGreater),
    Ordered(OpOrdered),
    Unordered(OpUnordered),
    LogicalEqual(OpLogicalEqual),
    LogicalNotEqual(OpLogicalNotEqual),
    LogicalOr(OpLogicalOr),
    LogicalAnd(OpLogicalAnd),
    LogicalNot(OpLogicalNot),
    Select(OpSelect),
    IEqual(OpIEqual),
    INotEqual(OpINotEqual),
    UGreaterThan(OpUGreaterThan),
    SGreaterThan(OpSGreaterThan),
    UGreaterThanEqual(OpUGreaterThanEqual),
    SGreaterThanEqual(OpSGreaterThanEqual),
    ULessThan(OpULessThan),
    SLessThan(OpSLessThan),
    ULessThanEqual(OpULessThanEqual),
    SLessThanEqual(OpSLessThanEqual),
    FOrdEqual(OpFOrdEqual),
    FUnordEqual(OpFUnordEqual),
    FOrdNotEqual(OpFOrdNotEqual),
    FUnordNotEqual(OpFUnordNotEqual),
    FOrdLessThan(OpFOrdLessThan),
    FUnordLessThan(OpFUnordLessThan),
    FOrdGreaterThan(OpFOrdGreaterThan),
    FUnordGreaterThan(OpFUnordGreaterThan),
    FOrdLessThanEqual(OpFOrdLessThanEqual),
    FUnordLessThanEqual(OpFUnordLessThanEqual),
    FOrdGreaterThanEqual(OpFOrdGreaterThanEqual),
    FUnordGreaterThanEqual(OpFUnordGreaterThanEqual),
    Phi(OpPhi),
    LoopMerge(OpLoopMerge),
    SelectionMerge(OpSelectionMerge),
    Label(OpLabel),
    Branch(OpBranch),
    BranchConditional(OpBranchConditional),
    Switch(OpSwitch),
    Kill(OpKill),
    Return(OpReturn),
    ReturnValue(OpReturnValue),
    Unreachable(OpUnreachable),
    LifetimeStart(OpLifetimeStart),
    LifetimeStop(OpLifetimeStop),
    TerminateInvocation(OpTerminateInvocation),
    DemoteToHelperInvocation(OpDemoteToHelperInvocation),
    AtomicLoad(OpAtomicLoad),
    AtomicStore(OpAtomicStore),
    AtomicExchange(OpAtomicExchange),
    AtomicCompareExchange(OpAtomicCompareExchange),
    AtomicCompareExchangeWeak(OpAtomicCompareExchangeWeak),
    AtomicIIncrement(OpAtomicIIncrement),
    AtomicIDecrement(OpAtomicIDecrement),
    AtomicIAdd(OpAtomicIAdd),
    AtomicISub(OpAtomicISub),
    AtomicSMin(OpAtomicSMin),
    AtomicUMin(OpAtomicUMin),
    AtomicSMax(OpAtomicSMax),
    AtomicUMax(OpAtomicUMax),
    AtomicAnd(OpAtomicAnd),
    AtomicOr(OpAtomicOr),
    AtomicXor(OpAtomicXor),
    AtomicFlagTestAndSet(OpAtomicFlagTestAndSet),
    AtomicFlagClear(OpAtomicFlagClear),
    EmitMeshTasksEXT(OpEmitMeshTasksEXT),
    SetMeshOutputsEXT(OpSetMeshOutputsEXT),
}

impl Instruction {
    pub fn read(reader: &mut WordReader<'_>) -> Result<Self, Error> {
        let word = reader.next().unwrap();

        let opcode = word & 0xFFFF;
        let word_count = ((word & 0xFFFF_0000) >> 16) as u16;

        // The word count includes the first instruction word, which we have
        // already read.
        let mut reader = InstructionReader::new(reader, word_count.saturating_sub(1));

        macro_rules! expand_match {
            ($($opcode:tt => $variant:tt),* $(,)?) => {
                match spirv::Op::from_u32(opcode) {
                    $(
                        Some(spirv::Op::$opcode) => {
                            Parse::parse(&mut reader).map(Self::$variant)
                        }
                    )*
                    _ => Err(Error(ErrorImpl::UnknownOpcode(opcode))),
                }
            };
        }

        expand_match! {
            Nop => Nop,
            Undef => Undef,
            SizeOf => SizeOf,
            SourceContinued => SourceContinued,
            Source => Source,
            SourceExtension => SourceExtension,
            Name => Name,
            MemberName => MemberName,
            String => String,
            Line => Line,
            NoLine => NoLine,
            ModuleProcessed => ModuleProcessed,
            Decorate => Decorate,
            MemberDecorate => MemberDecorate,
            DecorationGroup => DecorationGroup,
            GroupDecorate => GroupDecorate,
            GroupMemberDecorate => GroupMemberDecorate,
            DecorateId => DecorateId,
            DecorateString => DecorateString,
            MemberDecorateString => MemberDecorateString,
            Extension => Extension,
            ExtInstImport => ExtInstImport,
            ExtInst => ExtInst,
            MemoryModel => MemoryModel,
            EntryPoint => EntryPoint,
            ExecutionMode => ExecutionMode,
            Capability => Capability,
            ExecutionModeId => ExecutionModeId,
            TypeVoid => TypeVoid,
            TypeBool => TypeBool,
            TypeInt => TypeInt,
            TypeFloat => TypeFloat,
            TypeVector => TypeVector,
            TypeMatrix => TypeMatrix,
            TypeImage => TypeImage,
            TypeSampler => TypeSampler,
            TypeSampledImage => TypeSampledImage,
            TypeArray => TypeArray,
            TypeRuntimeArray => TypeRuntimeArray,
            TypeStruct => TypeStruct,
            TypePointer => TypePointer,
            TypeFunction => TypeFunction,
            ConstantTrue => ConstantTrue,
            ConstantFalse => ConstantFalse,
            Constant => Constant,
            ConstantComposite => ConstantComposite,
            ConstantSampler => ConstantSampler,
            ConstantNull => ConstantNull,
            SpecConstantTrue => SpecConstantTrue,
            SpecConstantFalse => SpecConstantFalse,
            SpecConstant => SpecConstant,
            SpecConstantComposite => SpecConstantComposite,
            SpecConstantOp => SpecConstantOp,
            Variable => Variable,
            ImageTexelPointer => ImageTexelPointer,
            Load => Load,
            Store => Store,
            CopyMemory => CopyMemory,
            CopyMemorySized => CopyMemorySized,
            AccessChain => AccessChain,
            InBoundsAccessChain => InBoundsAccessChain,
            ArrayLength => ArrayLength,
            Function => Function,
            FunctionParameter => FunctionParameter,
            FunctionEnd => FunctionEnd,
            FunctionCall => FunctionCall,
            SampledImage => SampledImage,
            ImageSampleImplicitLod => ImageSampleImplicitLod,
            ImageSampleExplicitLod => ImageSampleExplicitLod,
            ImageSampleDrefImplicitLod => ImageSampleDrefImplicitLod,
            ImageSampleDrefExplicitLod => ImageSampleDrefExplicitLod,
            ImageSampleProjImplicitLod => ImageSampleProjImplicitLod,
            ImageSampleProjExplicitLod => ImageSampleProjExplicitLod,
            ImageSampleProjDrefImplicitLod => ImageSampleProjDrefImplicitLod,
            ImageSampleProjDrefExplicitLod => ImageSampleProjDrefExplicitLod,
            ImageFetch => ImageFetch,
            ImageRead => ImageRead,
            ImageWrite => ImageWrite,
            ConvertFToU => ConvertFToU,
            ConvertFToS => ConvertFToS,
            ConvertSToF => ConvertSToF,
            ConvertUToF => ConvertUToF,
            UConvert => UConvert,
            SConvert => SConvert,
            FConvert => FConvert,
            QuantizeToF16 => QuantizeToF16,
            SatConvertSToU => SatConvertSToU,
            SatConvertUToS => SatConvertUToS,
            Bitcast => Bitcast,
            VectorExtractDynamic => VectorExtractDynamic,
            VectorInsertDynamic => VectorInsertDynamic,
            VectorShuffle => VectorShuffle,
            CompositeConstruct => CompositeConstruct,
            CompositeExtract => CompositeExtract,
            CompositeInsert => CompositeInsert,
            CopyObject => CopyObject,
            Transpose => Transpose,
            CopyLogical => CopyLogical,
            SNegate => SNegate,
            FNegate => FNegate,
            IAdd => IAdd,
            FAdd => FAdd,
            ISub => ISub,
            FSub => FSub,
            IMul => IMul,
            FMul => FMul,
            UDiv => UDiv,
            SDiv => SDiv,
            FDiv => FDiv,
            UMod => UMod,
            SRem => SRem,
            SMod => SMod,
            FRem => FRem,
            FMod => FMod,
            VectorTimesScalar => VectorTimesScalar,
            MatrixTimesScalar => MatrixTimesScalar,
            VectorTimesMatrix => VectorTimesMatrix,
            MatrixTimesVector => MatrixTimesVector,
            MatrixTimesMatrix => MatrixTimesMatrix,
            OuterProduct => OuterProduct,
            Dot => Dot,
            IAddCarry => IAddCarry,
            ISubBorrow => ISubBorrow,
            UMulExtended => UMulExtended,
            SMulExtended => SMulExtended,
            SDot => SDot,
            UDot => UDot,
            SUDot => SUDot,
            SDotAccSat => SDotAccSat,
            UDotAccSat => UDotAccSat,
            SUDotAccSat => SUDotAccSat,
            ShiftRightLogical => ShiftRightLogical,
            ShiftRightArithmetic => ShiftRightArithmetic,
            ShiftLeftLogical => ShiftLeftLogical,
            BitwiseOr => BitwiseOr,
            BitwiseXor => BitwiseXor,
            BitwiseAnd => BitwiseAnd,
            Not => Not,
            BitFieldInsert => BitFieldInsert,
            BitFieldSExtract => BitFieldSExtract,
            BitFieldUExtract => BitFieldUExtract,
            BitReverse => BitReverse,
            BitCount => BitCount,
            Any => Any,
            All => All,
            IsNan => IsNan,
            IsInf => IsInf,
            IsFinite => IsFinite,
            IsNormal => IsNormal,
            SignBitSet => SignBitSet,
            LessOrGreater => LessOrGreater,
            Ordered => Ordered,
            Unordered => Unordered,
            LogicalEqual => LogicalEqual,
            LogicalNotEqual => LogicalNotEqual,
            LogicalOr => LogicalOr,
            LogicalAnd => LogicalAnd,
            LogicalNot => LogicalNot,
            Select => Select,
            IEqual => IEqual,
            INotEqual => INotEqual,
            UGreaterThan => UGreaterThan,
            SGreaterThan => SGreaterThan,
            UGreaterThanEqual => UGreaterThanEqual,
            SGreaterThanEqual => SGreaterThanEqual,
            ULessThan => ULessThan,
            SLessThan => SLessThan,
            ULessThanEqual => ULessThanEqual,
            SLessThanEqual => SLessThanEqual,
            FOrdEqual => FOrdEqual,
            FUnordEqual => FUnordEqual,
            FOrdNotEqual => FOrdNotEqual,
            FUnordNotEqual => FUnordNotEqual,
            FOrdLessThan => FOrdLessThan,
            FUnordLessThan => FUnordLessThan,
            FOrdGreaterThan => FOrdGreaterThan,
            FUnordGreaterThan => FUnordGreaterThan,
            FOrdLessThanEqual => FOrdLessThanEqual,
            FUnordLessThanEqual => FUnordLessThanEqual,
            FOrdGreaterThanEqual => FOrdGreaterThanEqual,
            FUnordGreaterThanEqual => FUnordGreaterThanEqual,
            Phi => Phi,
            LoopMerge => LoopMerge,
            SelectionMerge => SelectionMerge,
            Label => Label,
            Branch => Branch,
            BranchConditional => BranchConditional,
            Switch => Switch,
            Kill => Kill,
            Return => Return,
            ReturnValue => ReturnValue,
            Unreachable => Unreachable,
            LifetimeStart => LifetimeStart,
            LifetimeStop => LifetimeStop,
            TerminateInvocation => TerminateInvocation,
            DemoteToHelperInvocation => DemoteToHelperInvocation,
            AtomicLoad => AtomicLoad,
            AtomicStore => AtomicStore,
            AtomicExchange => AtomicExchange,
            AtomicCompareExchange => AtomicCompareExchange,
            AtomicCompareExchangeWeak => AtomicCompareExchangeWeak,
            AtomicIIncrement => AtomicIIncrement,
            AtomicIDecrement => AtomicIDecrement,
            AtomicIAdd => AtomicIAdd,
            AtomicISub => AtomicISub,
            AtomicSMin => AtomicSMin,
            AtomicUMin => AtomicUMin,
            AtomicSMax => AtomicSMax,
            AtomicUMax => AtomicUMax,
            AtomicAnd => AtomicAnd,
            AtomicOr => AtomicOr,
            AtomicXor => AtomicXor,
            AtomicFlagTestAndSet => AtomicFlagTestAndSet,
            AtomicFlagClear => AtomicFlagClear,
            EmitMeshTasksEXT => EmitMeshTasksEXT,
            SetMeshOutputsEXT => SetMeshOutputsEXT,
        }
    }

    pub fn write(&self, writer: &mut Vec<u32>) {
        let prev_len = writer.len();
        writer.push(0);

        macro_rules! expand_match {
            ($($opcode:tt => $variant:tt),* $(,)?) => {
                match self {
                    $(
                        Self::$variant(v) => {
                            v.write(writer);
                            spirv::Op::$opcode
                        }
                    )*
                }
            };
        }

        let opcode = expand_match! {
            Nop => Nop,
            Undef => Undef,
            SizeOf => SizeOf,
            SourceContinued => SourceContinued,
            Source => Source,
            SourceExtension => SourceExtension,
            Name => Name,
            MemberName => MemberName,
            String => String,
            Line => Line,
            NoLine => NoLine,
            ModuleProcessed => ModuleProcessed,
            Decorate => Decorate,
            MemberDecorate => MemberDecorate,
            DecorationGroup => DecorationGroup,
            GroupDecorate => GroupDecorate,
            GroupMemberDecorate => GroupMemberDecorate,
            DecorateId => DecorateId,
            DecorateString => DecorateString,
            MemberDecorateString => MemberDecorateString,
            Extension => Extension,
            ExtInstImport => ExtInstImport,
            ExtInst => ExtInst,
            MemoryModel => MemoryModel,
            EntryPoint => EntryPoint,
            ExecutionMode => ExecutionMode,
            Capability => Capability,
            ExecutionModeId => ExecutionModeId,
            TypeVoid => TypeVoid,
            TypeBool => TypeBool,
            TypeInt => TypeInt,
            TypeFloat => TypeFloat,
            TypeVector => TypeVector,
            TypeMatrix => TypeMatrix,
            TypeImage => TypeImage,
            TypeSampler => TypeSampler,
            TypeSampledImage => TypeSampledImage,
            TypeArray => TypeArray,
            TypeRuntimeArray => TypeRuntimeArray,
            TypeStruct => TypeStruct,
            TypePointer => TypePointer,
            TypeFunction => TypeFunction,
            ConstantTrue => ConstantTrue,
            ConstantFalse => ConstantFalse,
            Constant => Constant,
            ConstantComposite => ConstantComposite,
            ConstantSampler => ConstantSampler,
            ConstantNull => ConstantNull,
            SpecConstantTrue => SpecConstantTrue,
            SpecConstantFalse => SpecConstantFalse,
            SpecConstant => SpecConstant,
            SpecConstantComposite => SpecConstantComposite,
            SpecConstantOp => SpecConstantOp,
            Variable => Variable,
            ImageTexelPointer => ImageTexelPointer,
            Load => Load,
            Store => Store,
            CopyMemory => CopyMemory,
            CopyMemorySized => CopyMemorySized,
            AccessChain => AccessChain,
            InBoundsAccessChain => InBoundsAccessChain,
            ArrayLength => ArrayLength,
            Function => Function,
            FunctionParameter => FunctionParameter,
            FunctionEnd => FunctionEnd,
            FunctionCall => FunctionCall,
            SampledImage => SampledImage,
            ImageSampleImplicitLod => ImageSampleImplicitLod,
            ImageSampleExplicitLod => ImageSampleExplicitLod,
            ImageSampleDrefImplicitLod => ImageSampleDrefImplicitLod,
            ImageSampleDrefExplicitLod => ImageSampleDrefExplicitLod,
            ImageSampleProjImplicitLod => ImageSampleProjImplicitLod,
            ImageSampleProjExplicitLod => ImageSampleProjExplicitLod,
            ImageSampleProjDrefImplicitLod => ImageSampleProjDrefImplicitLod,
            ImageSampleProjDrefExplicitLod => ImageSampleProjDrefExplicitLod,
            ImageFetch => ImageFetch,
            ImageRead => ImageRead,
            ImageWrite => ImageWrite,
            ConvertFToU => ConvertFToU,
            ConvertFToS => ConvertFToS,
            ConvertSToF => ConvertSToF,
            ConvertUToF => ConvertUToF,
            UConvert => UConvert,
            SConvert => SConvert,
            FConvert => FConvert,
            QuantizeToF16 => QuantizeToF16,
            SatConvertSToU => SatConvertSToU,
            SatConvertUToS => SatConvertUToS,
            Bitcast => Bitcast,
            VectorExtractDynamic => VectorExtractDynamic,
            VectorInsertDynamic => VectorInsertDynamic,
            VectorShuffle => VectorShuffle,
            CompositeConstruct => CompositeConstruct,
            CompositeExtract => CompositeExtract,
            CompositeInsert => CompositeInsert,
            CopyObject => CopyObject,
            Transpose => Transpose,
            CopyLogical => CopyLogical,
            SNegate => SNegate,
            FNegate => FNegate,
            IAdd => IAdd,
            FAdd => FAdd,
            ISub => ISub,
            FSub => FSub,
            IMul => IMul,
            FMul => FMul,
            UDiv => UDiv,
            SDiv => SDiv,
            FDiv => FDiv,
            UMod => UMod,
            SRem => SRem,
            SMod => SMod,
            FRem => FRem,
            FMod => FMod,
            VectorTimesScalar => VectorTimesScalar,
            MatrixTimesScalar => MatrixTimesScalar,
            VectorTimesMatrix => VectorTimesMatrix,
            MatrixTimesVector => MatrixTimesVector,
            MatrixTimesMatrix => MatrixTimesMatrix,
            OuterProduct => OuterProduct,
            Dot => Dot,
            IAddCarry => IAddCarry,
            ISubBorrow => ISubBorrow,
            UMulExtended => UMulExtended,
            SMulExtended => SMulExtended,
            SDot => SDot,
            UDot => UDot,
            SUDot => SUDot,
            SDotAccSat => SDotAccSat,
            UDotAccSat => UDotAccSat,
            SUDotAccSat => SUDotAccSat,
            ShiftRightLogical => ShiftRightLogical,
            ShiftRightArithmetic => ShiftRightArithmetic,
            ShiftLeftLogical => ShiftLeftLogical,
            BitwiseOr => BitwiseOr,
            BitwiseXor => BitwiseXor,
            BitwiseAnd => BitwiseAnd,
            Not => Not,
            BitFieldInsert => BitFieldInsert,
            BitFieldSExtract => BitFieldSExtract,
            BitFieldUExtract => BitFieldUExtract,
            BitReverse => BitReverse,
            BitCount => BitCount,
            Any => Any,
            All => All,
            IsNan => IsNan,
            IsInf => IsInf,
            IsFinite => IsFinite,
            IsNormal => IsNormal,
            SignBitSet => SignBitSet,
            LessOrGreater => LessOrGreater,
            Ordered => Ordered,
            Unordered => Unordered,
            LogicalEqual => LogicalEqual,
            LogicalNotEqual => LogicalNotEqual,
            LogicalOr => LogicalOr,
            LogicalAnd => LogicalAnd,
            LogicalNot => LogicalNot,
            Select => Select,
            IEqual => IEqual,
            INotEqual => INotEqual,
            UGreaterThan => UGreaterThan,
            SGreaterThan => SGreaterThan,
            UGreaterThanEqual => UGreaterThanEqual,
            SGreaterThanEqual => SGreaterThanEqual,
            ULessThan => ULessThan,
            SLessThan => SLessThan,
            ULessThanEqual => ULessThanEqual,
            SLessThanEqual => SLessThanEqual,
            FOrdEqual => FOrdEqual,
            FUnordEqual => FUnordEqual,
            FOrdNotEqual => FOrdNotEqual,
            FUnordNotEqual => FUnordNotEqual,
            FOrdLessThan => FOrdLessThan,
            FUnordLessThan => FUnordLessThan,
            FOrdGreaterThan => FOrdGreaterThan,
            FUnordGreaterThan => FUnordGreaterThan,
            FOrdLessThanEqual => FOrdLessThanEqual,
            FUnordLessThanEqual => FUnordLessThanEqual,
            FOrdGreaterThanEqual => FOrdGreaterThanEqual,
            FUnordGreaterThanEqual => FUnordGreaterThanEqual,
            Phi => Phi,
            LoopMerge => LoopMerge,
            SelectionMerge => SelectionMerge,
            Label => Label,
            Branch => Branch,
            BranchConditional => BranchConditional,
            Switch => Switch,
            Kill => Kill,
            Return => Return,
            ReturnValue => ReturnValue,
            Unreachable => Unreachable,
            LifetimeStart => LifetimeStart,
            LifetimeStop => LifetimeStop,
            TerminateInvocation => TerminateInvocation,
            DemoteToHelperInvocation => DemoteToHelperInvocation,
            AtomicLoad => AtomicLoad,
            AtomicStore => AtomicStore,
            AtomicExchange => AtomicExchange,
            AtomicCompareExchange => AtomicCompareExchange,
            AtomicCompareExchangeWeak => AtomicCompareExchangeWeak,
            AtomicIIncrement => AtomicIIncrement,
            AtomicIDecrement => AtomicIDecrement,
            AtomicIAdd => AtomicIAdd,
            AtomicISub => AtomicISub,
            AtomicSMin => AtomicSMin,
            AtomicUMin => AtomicUMin,
            AtomicSMax => AtomicSMax,
            AtomicUMax => AtomicUMax,
            AtomicAnd => AtomicAnd,
            AtomicOr => AtomicOr,
            AtomicXor => AtomicXor,
            AtomicFlagTestAndSet => AtomicFlagTestAndSet,
            AtomicFlagClear => AtomicFlagClear,
            EmitMeshTasksEXT => EmitMeshTasksEXT,
            SetMeshOutputsEXT => SetMeshOutputsEXT,
        };

        // The word count is the number of words we have written
        // since this function invocation.
        let word_count = (writer.len() - prev_len) as u32;
        debug_assert_ne!(word_count, 0);
        writer[prev_len] = opcode as u32 | (word_count << 16);
        debug_assert_ne!(writer[prev_len], 0);
    }
}

#[derive(Debug)]
pub struct InstructionReader<'a, 'b> {
    reader: &'b mut WordReader<'a>,
    len: u16,
    consumed: u16,
    op_name: &'static str,
}

impl<'a, 'b> InstructionReader<'a, 'b> {
    fn new(reader: &'b mut WordReader<'a>, len: u16) -> Self {
        Self {
            reader,
            len,
            consumed: 0,
            op_name: "",
        }
    }

    fn set_op_name(&mut self, name: &'static str) {
        self.op_name = name;
    }

    /// Consumes the next argument if it exists.
    fn consume(&mut self) -> Result<u32, Error> {
        if self.len == 0 || self.reader.len() == 0 {
            Err(Error(ErrorImpl::InvalidArgumentCount(
                InvalidArgumentCount {
                    op: self.op_name,
                    required: self.consumed as usize + 1,
                    found: self.consumed as usize,
                    variable: false,
                },
            )))
        } else {
            self.len -= 1;
            self.consumed += 1;
            Ok(self.reader.next().unwrap())
        }
    }

    fn is_empty(&self) -> bool {
        self.len == 0 || self.reader.len() == 0
    }
}

pub trait Parse: Sized {
    fn parse(reader: &mut InstructionReader<'_, '_>) -> Result<Self, Error>;

    fn write(&self, writer: &mut Vec<u32>);
}

impl Parse for u32 {
    fn parse(reader: &mut InstructionReader<'_, '_>) -> Result<Self, Error> {
        reader.consume()
    }

    fn write(&self, writer: &mut Vec<u32>) {
        writer.push(*self);
    }
}

impl Parse for bool {
    fn parse(reader: &mut InstructionReader<'_, '_>) -> Result<Self, Error> {
        u32::parse(reader).map(|v| v != 0)
    }

    fn write(&self, writer: &mut Vec<u32>) {
        u32::from(*self).write(writer);
    }
}

impl Parse for Id {
    fn parse(reader: &mut InstructionReader<'_, '_>) -> Result<Self, Error> {
        reader.consume().map(Self)
    }

    fn write(&self, writer: &mut Vec<u32>) {
        self.0.write(writer);
    }
}

impl<T> Parse for Vec<T>
where
    T: Parse,
{
    fn parse(reader: &mut InstructionReader<'_, '_>) -> Result<Self, Error> {
        let mut elems = Vec::new();
        while !reader.is_empty() {
            elems.push(T::parse(reader)?);
        }

        Ok(elems)
    }

    fn write(&self, writer: &mut Vec<u32>) {
        for elem in self {
            elem.write(writer);
        }
    }
}

impl<T> Parse for Option<T>
where
    T: Parse,
{
    fn parse(reader: &mut InstructionReader<'_, '_>) -> Result<Self, Error> {
        if reader.is_empty() {
            Ok(None)
        } else {
            T::parse(reader).map(Some)
        }
    }

    fn write(&self, writer: &mut Vec<u32>) {
        if let Some(v) = self {
            v.write(writer);
        }
    }
}

impl Parse for String {
    fn parse(reader: &mut InstructionReader<'_, '_>) -> Result<Self, Error> {
        let mut bytes = Vec::new();
        'outer: loop {
            let word = reader.consume()?;
            for byte in word.to_le_bytes() {
                if byte == 0 {
                    break 'outer;
                }

                bytes.push(byte);
            }
        }

        Self::from_utf8(bytes).map_err(|v| Error(ErrorImpl::InvalidString(v)))
    }

    fn write(&self, writer: &mut Vec<u32>) {
        for chunk in self.as_bytes().chunks(size_of::<u32>()) {
            let mut arr = [0; 4];
            arr[..chunk.len()].copy_from_slice(chunk);
            writer.push(u32::from_le_bytes(arr));
        }

        // The string must be nul terminated, which is indicated
        // by a single nul byte.
        // If all words are occupied with string data we need to
        // extend with a 4 nul bytes.
        if self.as_bytes().len() % 4 == 0 {
            writer.push(0);
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id(pub u32);

impl Id {
    pub const DUMMY: Self = Self(u32::MAX);
}

macro_rules! spirv_enum {
    ($enum_ty:ty) => {
        impl Parse for $enum_ty {
            fn parse(reader: &mut InstructionReader<'_, '_>) -> Result<Self, Error> {
                let word = reader.consume()?;
                match Self::from_u32(word) {
                    Some(value) => Ok(value),
                    None => {
                        return Err(Error(ErrorImpl::UnknownEnumValue(
                            stringify!($enum_ty),
                            word,
                        )));
                    }
                }
            }

            fn write(&self, writer: &mut Vec<u32>) {
                writer.push(*self as u32);
            }
        }
    };
}

macro_rules! spirv_bitflags {
    ($flags_ty:ty) => {
        impl Parse for $flags_ty {
            fn parse(reader: &mut InstructionReader<'_, '_>) -> Result<Self, Error> {
                let word = reader.consume()?;
                match Self::from_bits(word) {
                    Some(value) => Ok(value),
                    None => {
                        return Err(Error(ErrorImpl::UnknownEnumValue(
                            stringify!($flags_ty),
                            word,
                        )));
                    }
                }
            }

            fn write(&self, writer: &mut Vec<u32>) {
                writer.push(self.bits());
            }
        }
    };
}

spirv_enum!(spirv::Decoration);
spirv_enum!(ExecutionModel);
spirv_enum!(Dim);
spirv_enum!(ImageFormat);
spirv_enum!(StorageClass);
spirv_enum!(SamplerAddressingMode);
spirv_enum!(SamplerFilterMode);
spirv_enum!(PackedVectorFormat);
spirv_enum!(SourceLanguage);
spirv_enum!(AddressingModel);
spirv_enum!(MemoryModel);
spirv_enum!(ExecutionMode);
spirv_enum!(Capability);
spirv_enum!(BuiltIn);
spirv_enum!(FPRoundingMode);

spirv_bitflags!(FunctionControl);
spirv_bitflags!(LoopControl);
spirv_bitflags!(SelectionControl);
spirv_bitflags!(MemorySemantics);
spirv_bitflags!(FPFastMathMode);

macro_rules! spirv_op {
    ($(#[$($struct_attr:tt)*])* $struct_vis:vis struct $struct_id:ident
        {
            $(
                $(#[$($field_attr:tt)*])* $field_vis:vis $field_id:ident : $field_ty:ty,
            )*
        }

    ) => {
        // Recreate the struct definition.
        $(
            #[ $( $struct_attr )* ]
        )*
        $struct_vis struct $struct_id {
            $(
                $(#[$($field_attr)*])* $field_vis $field_id: $field_ty,
            )*
        }


        impl Parse for $struct_id {
            fn parse(reader: &mut InstructionReader<'_, '_>) -> Result<Self, Error> {
                reader.set_op_name(stringify!($struct_id));

                Ok(Self {
                    $(
                        $field_id: <$field_ty as Parse>::parse(reader)?,
                    )*
                })
            }

            #[allow(unused)]
            fn write(&self, writer: &mut Vec<u32>) {
                $(
                    self.$field_id.write(writer);
                )*
            }
        }
    };
    ($(#[$($struct_attr:tt)*])* $struct_vis:vis struct $struct_id:ident;) => {

        // Recreate the struct definition.
        $(
            #[ $( $struct_attr )* ]
        )*
        $struct_vis struct $struct_id;

        impl Parse for $struct_id {
            fn parse(reader: &mut InstructionReader<'_, '_>) -> Result<Self, Error> {
                reader.set_op_name(stringify!($struct_id));
                Ok(Self)
            }

            fn write(&self, _writer: &mut Vec<u32>) {}
        }
    }
}

// ==================================
// === Miscellaneous Instructions ===
// ==================================

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpNop {}
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpUndef {
        pub result_type: Id,
        pub result: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpSizeOf {
        pub result_type: Id,
        pub result: Id,
        pub pointer: Id,
    }
}

// ==========================
// === Debug Instructions ===
// ==========================

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpSourceContinued {
        pub continued_source: String,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpSource {
        pub source_language: SourceLanguage,
        pub file: Option<Id>,
        pub source: Option<String>,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpSourceExtension {
        pub extension: String,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpName {
        pub target: Id,
        pub name: String,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpMemberName {
        pub type_: Id,
        pub member: u32,
        pub name: String,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpString {
        pub result: Id,
        pub string: String,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpLine {
        pub file: Id,
        pub line: u32,
        pub column: u32,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpNoLine {
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpModuleProcessed {
        pub process: String,
    }
}

// ===============================
// === Annotation Instructions ===
// ===============================

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpDecorate {
        pub target: Id,
        pub decoration: Decoration,
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Decoration {
    RelaxedPrecision,
    Block,
    RowMajor,
    ColMajor,
    ArrayStride(u32),
    MatrixStride(u32),
    Builtin(BuiltIn),
    NoPerspective,
    Flat,
    Restrict,
    Aliased,
    Volatile,
    Constant,
    Coherent,
    NonWritable,
    NonReadable,
    Uniform,
    SaturatedConversion,
    Location(u32),
    Component(u32),
    Index(u32),
    Binding(u32),
    DescriptorSet(u32),
    Offset(u32),
    FPRoundingMode(FPRoundingMode),
    FPFastMathMode(FPFastMathMode),
    NoContraction,
    InputAttachmentIndex(u32),
    Alignment(u32),
    NonUniform,
}

impl Parse for Decoration {
    fn parse(reader: &mut InstructionReader<'_, '_>) -> Result<Self, Error> {
        let dec = spirv::Decoration::parse(reader)?;
        match dec {
            spirv::Decoration::RelaxedPrecision => Ok(Self::RelaxedPrecision),
            spirv::Decoration::Block => Ok(Self::Block),
            spirv::Decoration::RowMajor => Ok(Self::RowMajor),
            spirv::Decoration::ColMajor => Ok(Self::ColMajor),
            spirv::Decoration::ArrayStride => {
                let stride = u32::parse(reader)?;
                Ok(Self::ArrayStride(stride))
            }
            spirv::Decoration::MatrixStride => {
                let stride = u32::parse(reader)?;
                Ok(Self::MatrixStride(stride))
            }
            spirv::Decoration::BuiltIn => {
                let builtin = BuiltIn::parse(reader)?;
                Ok(Self::Builtin(builtin))
            }
            spirv::Decoration::NoPerspective => Ok(Self::NoPerspective),
            spirv::Decoration::Flat => Ok(Self::Flat),
            spirv::Decoration::Restrict => Ok(Self::Restrict),
            spirv::Decoration::Aliased => Ok(Self::Aliased),
            spirv::Decoration::Volatile => Ok(Self::Volatile),
            spirv::Decoration::Constant => Ok(Self::Constant),
            spirv::Decoration::Coherent => Ok(Self::Coherent),
            spirv::Decoration::NonWritable => Ok(Self::NonWritable),
            spirv::Decoration::NonReadable => Ok(Self::NonReadable),
            spirv::Decoration::Uniform => Ok(Self::Uniform),
            spirv::Decoration::SaturatedConversion => Ok(Self::SaturatedConversion),
            spirv::Decoration::Location => {
                let location = u32::parse(reader)?;
                Ok(Self::Location(location))
            }
            spirv::Decoration::Component => {
                let component = u32::parse(reader)?;
                Ok(Self::Component(component))
            }
            spirv::Decoration::Index => {
                let index = u32::parse(reader)?;
                Ok(Self::Index(index))
            }
            spirv::Decoration::Binding => {
                let id = u32::parse(reader)?;
                Ok(Self::Binding(id))
            }
            spirv::Decoration::DescriptorSet => {
                let id = u32::parse(reader)?;
                Ok(Self::DescriptorSet(id))
            }
            spirv::Decoration::Offset => {
                let offset = u32::parse(reader)?;
                Ok(Self::Offset(offset))
            }
            spirv::Decoration::FPRoundingMode => {
                let mode = FPRoundingMode::parse(reader)?;
                Ok(Self::FPRoundingMode(mode))
            }
            spirv::Decoration::FPFastMathMode => {
                let mode = FPFastMathMode::parse(reader)?;
                Ok(Self::FPFastMathMode(mode))
            }
            spirv::Decoration::NoContraction => Ok(Self::NoContraction),
            spirv::Decoration::InputAttachmentIndex => {
                let index = u32::parse(reader)?;
                Ok(Self::InputAttachmentIndex(index))
            }
            spirv::Decoration::Alignment => {
                let align = u32::parse(reader)?;
                Ok(Self::Alignment(align))
            }
            spirv::Decoration::NonUniform => Ok(Self::NonUniform),
            _ => Err(Error(ErrorImpl::UnknownDecoration(dec))),
        }
    }

    fn write(&self, writer: &mut Vec<u32>) {
        match self {
            Self::RelaxedPrecision => {
                spirv::Decoration::RelaxedPrecision.write(writer);
            }
            Self::Block => {
                spirv::Decoration::Block.write(writer);
            }

            Self::RowMajor => {
                spirv::Decoration::RowMajor.write(writer);
            }
            Self::ColMajor => {
                spirv::Decoration::ColMajor.write(writer);
            }
            Self::ArrayStride(stride) => {
                spirv::Decoration::ArrayStride.write(writer);
                stride.write(writer);
            }
            Self::MatrixStride(stride) => {
                spirv::Decoration::MatrixStride.write(writer);
                stride.write(writer);
            }
            Self::Builtin(builtin) => {
                spirv::Decoration::BuiltIn.write(writer);
                builtin.write(writer);
            }
            Self::NoPerspective => {
                spirv::Decoration::NoPerspective.write(writer);
            }
            Self::Flat => {
                spirv::Decoration::Flat.write(writer);
            }
            Self::Restrict => {
                spirv::Decoration::Restrict.write(writer);
            }
            Self::Aliased => {
                spirv::Decoration::Aliased.write(writer);
            }
            Self::Volatile => {
                spirv::Decoration::Volatile.write(writer);
            }
            Self::Constant => {
                spirv::Decoration::Constant.write(writer);
            }
            Self::Coherent => {
                spirv::Decoration::Coherent.write(writer);
            }
            Self::NonWritable => {
                spirv::Decoration::NonWritable.write(writer);
            }
            Self::NonReadable => {
                spirv::Decoration::NonReadable.write(writer);
            }
            Self::Uniform => {
                spirv::Decoration::Uniform.write(writer);
            }
            Self::SaturatedConversion => {
                spirv::Decoration::SaturatedConversion.write(writer);
            }
            Self::Location(location) => {
                spirv::Decoration::Location.write(writer);
                location.write(writer);
            }
            Self::Component(component) => {
                spirv::Decoration::Component.write(writer);
                component.write(writer);
            }
            Self::Index(index) => {
                spirv::Decoration::Index.write(writer);
                index.write(writer);
            }
            Self::Binding(id) => {
                spirv::Decoration::Binding.write(writer);
                id.write(writer);
            }
            Self::DescriptorSet(id) => {
                spirv::Decoration::DescriptorSet.write(writer);
                id.write(writer);
            }
            Self::Offset(offset) => {
                spirv::Decoration::Offset.write(writer);
                offset.write(writer);
            }
            Self::FPRoundingMode(mode) => {
                spirv::Decoration::FPRoundingMode.write(writer);
                mode.write(writer);
            }
            Self::FPFastMathMode(mode) => {
                spirv::Decoration::FPFastMathMode.write(writer);
                mode.write(writer);
            }
            Self::NoContraction => {
                spirv::Decoration::NoContraction.write(writer);
            }
            Self::InputAttachmentIndex(index) => {
                spirv::Decoration::InputAttachmentIndex.write(writer);
                index.write(writer);
            }
            Self::Alignment(align) => {
                spirv::Decoration::Alignment.write(writer);
                align.write(writer);
            }
            Self::NonUniform => {
                spirv::Decoration::NonUniform.write(writer);
            }
        }
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpMemberDecorate {
        pub structure_type: Id,
        pub member: u32,
        pub decoration: Decoration,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpDecorationGroup {
        pub result: Id,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpGroupDecorate {
        pub decoration_group: Id,
        pub targets: Vec<Id>,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpGroupMemberDecorate {
        pub decoration_group: Id,
        // FIXME: Change to Vec<(Id, u32)>.
        pub targets: Vec<u32>,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpDecorateId {
        pub target: Id,
        pub decoration: Decoration,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpDecorateString {
        pub target: Id,
        pub decoration: Decoration,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpMemberDecorateString {
        pub target: Id,
        pub decoration: Decoration,
    }
}

// ==============================
// === Extension Instructions ===
// ==============================

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpExtension {
        pub name: String,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpExtInstImport {
        pub result: Id,
        pub name: String,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpExtInst {
        pub result_type: Id,
        pub result: Id,
        pub set: Id,
        pub instruction: u32,
        pub operands: Vec<u32>,
    }
}

// =================================
// === Mode-Setting Instructions ===
// =================================

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpMemoryModel {
        pub addressing_model: AddressingModel,
        pub memory_model: MemoryModel,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpEntryPoint {
        pub execution_model: ExecutionModel,
        pub entry_point: Id,
        pub name: String,
        pub interface: Vec<Id>,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpExecutionMode {
        pub entry_point: Id,
        pub mode: ExecutionMode,
        pub operands: Vec<u32>,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpCapability {
        pub capability: Capability,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpExecutionModeId {
        pub entry_point: Id,
        pub mode: ExecutionMode,
        pub operands: Vec<Id>,
    }
}

// =====================================
// === Type-Declaration Instructions ===
// =====================================

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpTypeVoid {
        pub result: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpTypeBool {
        pub result: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpTypeInt {
        pub result: Id,
        pub width: u32,
        pub is_signed: bool,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpTypeFloat {
        pub result: Id,
        pub width: u32,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpTypeVector {
        pub result: Id,
        pub component_type: Id,
        pub component_count: u32,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpTypeMatrix {
        pub result: Id,
        pub column_type: Id,
        pub column_count: u32,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpTypeImage {
        pub result: Id,
        pub sampled_type: Id,
        pub dim: Dim,
        pub depth: u32,
        pub arrayed: u32,
        pub ms: u32,
        pub sampled: u32,
        pub format: ImageFormat,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpTypeSampler {
        pub result: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpTypeSampledImage {
        pub result: Id,
        pub image_type: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpTypeArray {
        pub result: Id,
        pub element_type: Id,
        pub length: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpTypeRuntimeArray {
        pub result: Id,
        pub element_type: Id,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpTypeStruct {
        pub result: Id,
        pub members: Vec<Id>,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpTypePointer {
        pub result: Id,
        pub storage_class: StorageClass,
        pub type_: Id,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpTypeFunction {
        pub result: Id,
        pub return_type: Id,
        pub parameters: Vec<Id>,
    }
}

// ======================================
// === Constant-Creation Instructions ===
// ======================================

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpConstantTrue {
        pub result_type: Id,
        pub result: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpConstantFalse {
        pub result_type: Id,
        pub result: Id,
    }
}

spirv_op! {
    /// Declares a new constant value.
    #[derive(Clone, Debug)]
    pub struct OpConstant {
        pub result_type: Id,
        pub result: Id,
        /// The untyped value of the constant.
        ///
        /// The type depends on the type behind `result_type`.
        pub value: Vec<u32>,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpConstantComposite {
        pub result_type: Id,
        pub result: Id,
        pub constituents: Vec<Id>,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpConstantSampler {
        pub result_type: Id,
        pub result: Id,
        pub sampler_addressing_mode: SamplerAddressingMode,
        pub param: u32,
        pub sampler_filter_mode: SamplerFilterMode,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpConstantNull {
        pub result_type: Id,
        pub result: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpSpecConstantTrue {
        pub result_type: Id,
        pub result: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpSpecConstantFalse {
        pub result_type: Id,
        pub result: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpSpecConstant {
        pub result_type: Id,
        pub result: Id,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpSpecConstantComposite {
        pub result_type: Id,
        pub result: Id,
        pub constituents: Vec<Id>,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpSpecConstantOp {
        pub result_type: Id,
        pub result: Id,
        pub opcode: u32,
        pub operands: Vec<Id>,
    }
}

// ============================
// === Memory Instructions ===
// ============================

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpVariable {
        pub result_type: Id,
        pub result: Id,
        pub storage_class: StorageClass,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpImageTexelPointer {
        pub result_type: Id,
        pub result: Id,
        pub image: Id,
        pub corrdinate: Id,
        pub sample: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpLoad {
        pub result_type: Id,
        pub result: Id,
        pub pointer: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpStore {
        pub pointer: Id,
        pub object: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpCopyMemory {
        pub target: Id,
        pub source: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpCopyMemorySized {
        pub target: Id,
        pub source: Id,
        pub size: Id,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpAccessChain {
        pub result_type: Id,
        pub result: Id,
        pub base: Id,
        pub indices: Vec<Id>,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpInBoundsAccessChain {
        pub result_type: Id,
        pub result: Id,
        pub base: Id,
        pub indices: Vec<Id>,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpArrayLength {
        pub result_type: Id,
        pub result: Id,
        pub structure: Id,
        pub array_member: Id,
    }
}

// =============================
// === Function Instructions ===
// =============================

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpFunction {
        pub result_type: Id,
        pub result: Id,
        pub function_control: FunctionControl,
        pub function_type: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpFunctionParameter {
        pub result_type: Id,
        pub result: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpFunctionEnd;
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpFunctionCall {
        pub result_type: Id,
        pub result: Id,
        pub function: Id,
        pub arguments: Vec<Id>,
    }
}

// ==========================
// === Image Instructions ===
// ==========================

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpSampledImage {
        pub result_type: Id,
        pub result: Id,
        pub image: Id,
        pub sampler: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpImageSampleImplicitLod {
        pub result_type: Id,
        pub result: Id,
        pub sampled_image: Id,
        pub coordinate: Id,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpImageSampleExplicitLod {
        pub result_type: Id,
        pub result: Id,
        pub sampled_image: Id,
        pub coordinate: Id,
        pub operands: Vec<u32>,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpImageSampleDrefImplicitLod {
        pub result_type: Id,
        pub result: Id,
        pub sampled_image: Id,
        pub coordinate: Id,
        pub dref: Id,
        pub operands: Vec<u32>,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpImageSampleDrefExplicitLod {
        pub result_type: Id,
        pub result: Id,
        pub sampled_image: Id,
        pub coordinate: Id,
        pub dref: Id,
        pub operands: Vec<u32>,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpImageSampleProjImplicitLod {
        pub result_type: Id,
        pub result: Id,
        pub sampled_image: Id,
        pub coordinate: Id,
        pub operands: Vec<u32>,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpImageSampleProjExplicitLod {
        pub result_type: Id,
        pub result: Id,
        pub sampled_image: Id,
        pub coordinate: Id,
        pub operands: Vec<u32>,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpImageSampleProjDrefImplicitLod {
        pub result_type: Id,
        pub result: Id,
        pub sampled_image: Id,
        pub coordinate: Id,
        pub dref: Id,
        pub operands: Vec<u32>,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpImageSampleProjDrefExplicitLod {
        pub result_type: Id,
        pub result: Id,
        pub sampled_image: Id,
        pub coordinate: Id,
        pub dref: Id,
        pub operands: Vec<u32>,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpImageFetch {
        pub result_type: Id,
        pub result: Id,
        pub image: Id,
        pub coordinate: Id,
        pub operands: Vec<u32>,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpImageRead {
        pub result_type: Id,
        pub result: Id,
        pub image: Id,
        pub coordinate: Id,
        pub operands: Vec<u32>,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpImageWrite {
        pub image: Id,
        pub coordinate: Id,
        pub texel: Id,
        pub operands: Vec<u32>,
    }
}

// ===============================
// === Conversion Instructions ===
// ===============================

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpConvertFToU {
        pub result_type: Id,
        pub result: Id,
        pub float_value: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpConvertFToS {
        pub result_type: Id,
        pub result: Id,
        pub float_value: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpConvertSToF {
        pub result_type: Id,
        pub result: Id,
        pub signed_value: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpConvertUToF {
        pub result_type: Id,
        pub result: Id,
        pub unsigned_value: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpUConvert {
        pub result_type: Id,
        pub result: Id,
        pub unsigned_value: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpSConvert {
        pub result_type: Id,
        pub result: Id,
        pub signed_value: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpFConvert {
        pub result_type: Id,
        pub result: Id,
        pub float_value: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpQuantizeToF16 {
        pub result_type: Id,
        pub result: Id,
        pub value: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpConvertPtrToU {
        pub result_type: Id,
        pub result: Id,
        pub pointer: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpSatConvertSToU {
        pub result_type: Id,
        pub result: Id,
        pub signed_value: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpSatConvertUToS {
        pub result_type: Id,
        pub result: Id,
        pub unsigned_value: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpBitcast {
        pub result_type: Id,
        pub result: Id,
        pub operand: Id,
    }
}

// ==============================
// === Composite Instructions ===
// ==============================

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpVectorExtractDynamic {
        pub result_type: Id,
        pub result: Id,
        pub vector: Id,
        pub index: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpVectorInsertDynamic {
        pub result_type: Id,
        pub result: Id,
        pub vector: Id,
        pub component: Id,
        pub index: Id,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpVectorShuffle {
        pub result_type: Id,
        pub result: Id,
        pub vector1: Id,
        pub vector2: Id,
        pub components: Vec<u32>,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpCompositeConstruct {
        pub result_type: Id,
        pub result: Id,
        pub constituents: Vec<Id>,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpCompositeExtract {
        pub result_type: Id,
        pub result: Id,
        pub composite: Id,
        pub indices: Vec<u32>,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpCompositeInsert {
        pub result_type: Id,
        pub result: Id,
        pub object: Id,
        pub composite: Id,
        pub indices: Vec<u32>,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpCopyObject {
        pub result_type: Id,
        pub result: Id,
        pub operand: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpTranspose {
        pub result_type: Id,
        pub result: Id,
        pub matrix: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpCopyLogical {
        pub result_type: Id,
        pub result: Id,
        pub operand: Id,
    }
}

// ===============================
// === Arithmetic Instructions ===
// ===============================

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpSNegate {
        pub result_type: Id,
        pub result: Id,
        pub operand: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpFNegate {
        pub result_type: Id,
        pub result: Id,
        pub operand: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpIAdd {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpFAdd {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpISub {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpFSub {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpIMul {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpFMul {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpUDiv {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpSDiv {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpFDiv {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpUMod {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpSRem {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpSMod {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpFRem {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpFMod {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpVectorTimesScalar {
        pub result_type: Id,
        pub result: Id,
        pub vector: Id,
        pub scalar: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpMatrixTimesScalar {
        pub result_type: Id,
        pub result: Id,
        pub matrix: Id,
        pub scalar: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpVectorTimesMatrix {
        pub result_type: Id,
        pub result: Id,
        pub vector: Id,
        pub matrix: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpMatrixTimesVector {
        pub result_type: Id,
        pub result: Id,
        pub matrix: Id,
        pub vector: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpMatrixTimesMatrix {
        pub result_type: Id,
        pub result: Id,
        pub left_matrix: Id,
        pub right_matrix: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpOuterProduct {
        pub result_type: Id,
        pub result: Id,
        pub vector1: Id,
        pub vector2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpDot {
        pub result_type: Id,
        pub result: Id,
        pub vector1: Id,
        pub vector2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpIAddCarry {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpISubBorrow {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpUMulExtended {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpSMulExtended {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpSDot {
        pub result_type: Id,
        pub result: Id,
        pub vector1: Id,
        pub vector2: Id,
        pub packed_vector_format: Option<PackedVectorFormat>,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpUDot {
        pub result_type: Id,
        pub result: Id,
        pub vector1: Id,
        pub vector2: Id,
        pub packed_vector_format: Option<PackedVectorFormat>,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpSUDot {
        pub result_type: Id,
        pub result: Id,
        pub vector1: Id,
        pub vector2: Id,
        pub packed_vector_format: Option<PackedVectorFormat>,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpSDotAccSat {
        pub result_type: Id,
        pub result: Id,
        pub vector1: Id,
        pub vector2: Id,
        pub accumulator: Id,
        pub packed_vector_format: Option<PackedVectorFormat>,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpUDotAccSat {
        pub result_type: Id,
        pub result: Id,
        pub vector1: Id,
        pub vector2: Id,
        pub accumulator: Id,
        pub packed_vector_format: Option<PackedVectorFormat>,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpSUDotAccSat {
        pub result_type: Id,
        pub result: Id,
        pub vector1: Id,
        pub vector2: Id,
        pub accumulator: Id,
        pub packed_vector_format: Option<PackedVectorFormat>,
    }
}

// ========================
// === Bit Instructions ===
// ========================

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpShiftRightLogical {
        pub result_type: Id,
        pub result: Id,
        pub base: Id,
        pub shift: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpShiftRightArithmetic {
        pub result_type: Id,
        pub result: Id,
        pub base: Id,
        pub shift: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpShiftLeftLogical {
        pub result_type: Id,
        pub result: Id,
        pub base: Id,
        pub shift: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpBitwiseOr {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpBitwiseXor {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpBitwiseAnd {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpNot {
        pub result_type: Id,
        pub result: Id,
        pub operand: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpBitFieldInsert {
        pub result_type: Id,
        pub result: Id,
        pub base: Id,
        pub insert: Id,
        pub offset: Id,
        pub count: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpBitFieldSExtract {
        pub result_type: Id,
        pub result: Id,
        pub base: Id,
        pub offset: Id,
        pub count: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpBitFieldUExtract {
        pub result_type: Id,
        pub result: Id,
        pub base: Id,
        pub offset: Id,
        pub count: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpBitReverse {
        pub result_type: Id,
        pub result: Id,
        pub base: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpBitCount {
        pub result_type: Id,
        pub result: Id,
        pub base: Id,
    }
}

// ===========================================
// === Relational and Logical Instructions ===
// ===========================================

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpAny {
        pub result_type: Id,
        pub result: Id,
        pub vector: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpAll {
        pub result_type: Id,
        pub result: Id,
        pub vector: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpIsNan {
        pub result_type: Id,
        pub result: Id,
        pub x: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpIsInf {
        pub result_type: Id,
        pub result: Id,
        pub x: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpIsFinite {
        pub result_type: Id,
        pub result: Id,
        pub x: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpIsNormal {
        pub result_type: Id,
        pub result: Id,
        pub x: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpSignBitSet {
        pub result_type: Id,
        pub result: Id,
        pub x: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpLessOrGreater {
        pub result_type: Id,
        pub result: Id,
        pub x: Id,
        pub y: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpOrdered {
        pub result_type: Id,
        pub result: Id,
        pub x: Id,
        pub y: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpUnordered {
        pub result_type: Id,
        pub result: Id,
        pub x: Id,
        pub y: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpLogicalEqual {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpLogicalNotEqual {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpLogicalOr {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpLogicalAnd {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpLogicalNot {
        pub result_type: Id,
        pub result: Id,
        pub operand: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpSelect {
        pub result_type: Id,
        pub result: Id,
        pub condition: Id,
        pub object1: Id,
        pub object2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpIEqual {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpINotEqual {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpUGreaterThan {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpSGreaterThan {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpUGreaterThanEqual {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpSGreaterThanEqual {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpULessThan {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpSLessThan {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpULessThanEqual {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpSLessThanEqual {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpFOrdEqual {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpFUnordEqual {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpFOrdNotEqual {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpFUnordNotEqual {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpFOrdLessThan {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpFUnordLessThan {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpFOrdGreaterThan {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpFUnordGreaterThan {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpFOrdLessThanEqual {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpFUnordLessThanEqual {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpFOrdGreaterThanEqual {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpFUnordGreaterThanEqual {
        pub result_type: Id,
        pub result: Id,
        pub operand1: Id,
        pub operand2: Id,
    }
}

// =================================
// === Control-Flow Instructions ===
// =================================

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpPhi {
        pub result_type: Id,
        pub result: Id,
        /// FIXME: This should be Vec<(Id, Id)> or something.
        pub variable_parent_pairs: Vec<Id>,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpLoopMerge {
        pub merge_block: Id,
        pub continue_target: Id,
        pub loop_control: LoopControl,
        pub loop_control_params: Vec<u32>,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpSelectionMerge {
        pub merge_block: Id,
        pub selection_control: SelectionControl,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpLabel {
        pub result: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpBranch {
        pub target_label: Id,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpBranchConditional {
        pub condition: Id,
        pub true_label: Id,
        pub false_label: Id,
        pub branch_weights: Vec<u32>,
    }
}

spirv_op! {
    #[derive(Clone, Debug)]
    pub struct OpSwitch {
        pub selector: Id,
        pub default: Id,
        pub target: Vec<u32>,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpKill {
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpReturn {
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpReturnValue {
        pub value: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpUnreachable;
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpLifetimeStart {
        pub pointer: Id,
        pub size: u32,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpLifetimeStop {
        pub pointer: Id,
        pub size: u32,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpTerminateInvocation;
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpDemoteToHelperInvocation;
}

// ===========================
// === Atomic Instructions ===
// ===========================

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpAtomicLoad {
        pub result_type: Id,
        pub result: Id,
        pub pointer: Id,
        pub memory: Id,
        pub semantics: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpAtomicStore {
        pub pointer: Id,
        pub memory: Id,
        pub semantics: Id,
        pub value: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpAtomicExchange {
        pub result_type: Id,
        pub result: Id,
        pub pointer: Id,
        pub memory: Id,
        pub semantics: Id,
        pub value: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpAtomicCompareExchange {
        pub result_type: Id,
        pub result: Id,
        pub pointer: Id,
        pub memory: Id,
        pub equal: Id,
        pub unequal: Id,
        pub value: Id,
        pub comparator: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpAtomicCompareExchangeWeak {
        pub result_type: Id,
        pub result: Id,
        pub pointer: Id,
        pub memory: Id,
        pub equal: Id,
        pub unequal: Id,
        pub value: Id,
        pub comparator: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpAtomicIIncrement {
        pub result_type: Id,
        pub result: Id,
        pub pointer: Id,
        pub memory: Id,
        pub semantics: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpAtomicIDecrement {
        pub result_type: Id,
        pub result: Id,
        pub pointer: Id,
        pub memory: Id,
        pub semantics: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpAtomicIAdd {
        pub result_type: Id,
        pub result: Id,
        pub pointer: Id,
        pub memory: Id,
        pub semantics: Id,
        pub value: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpAtomicISub {
        pub result_type: Id,
        pub result: Id,
        pub pointer: Id,
        pub memory: Id,
        pub semantics: Id,
        pub value: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpAtomicSMin {
        pub result_type: Id,
        pub result: Id,
        pub pointer: Id,
        pub memory: Id,
        pub semantics: Id,
        pub value: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpAtomicUMin {
        pub result_type: Id,
        pub result: Id,
        pub pointer: Id,
        pub memory: Id,
        pub semantics: Id,
        pub value: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpAtomicSMax {
        pub result_type: Id,
        pub result: Id,
        pub pointer: Id,
        pub memory: Id,
        pub semantics: Id,
        pub value: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpAtomicUMax {
        pub result_type: Id,
        pub result: Id,
        pub pointer: Id,
        pub memory: Id,
        pub semantics: Id,
        pub value: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpAtomicAnd {
        pub result_type: Id,
        pub result: Id,
        pub pointer: Id,
        pub memory: Id,
        pub semantics: Id,
        pub value: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpAtomicOr {
        pub result_type: Id,
        pub result: Id,
        pub pointer: Id,
        pub memory: Id,
        pub semantics: Id,
        pub value: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpAtomicXor {
        pub result_type: Id,
        pub result: Id,
        pub pointer: Id,
        pub memory: Id,
        pub semantics: Id,
        pub value: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpAtomicFlagTestAndSet {
        pub result_type: Id,
        pub result: Id,
        pub pointer: Id,
        pub memory: Id,
        pub semantics: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpAtomicFlagClear {
        pub pointer: Id,
        pub memory: Id,
        pub semantics: Id,
    }
}

// ======================
// === MeshShadingEXT ===
// ======================

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpEmitMeshTasksEXT {
        pub group_count_x: u32,
        pub group_count_y: u32,
        pub group_count_z: u32,
        pub payload: Id,
    }
}

spirv_op! {
    #[derive(Copy, Clone, Debug)]
    pub struct OpSetMeshOutputsEXT {
        pub vertex_count: u32,
        pub primitive_count: u32,
    }
}
