#[allow(unused_imports)] // right now just used in feature flag
use core;
use alloc::{SliceWrapper, Allocator};
#[derive(Debug,Copy,Clone,Default)]
pub struct BlockSwitch(pub u8);
// Commands that can instantiate as a no-op should implement this.
pub trait Nop<T> {
    fn nop() -> T;
}

impl BlockSwitch {
    pub fn new(block_type: u8) -> Self {
        BlockSwitch(block_type)
    }
    pub fn block_type(&self) -> u8 {
        self.0
    }
}

#[derive(Debug,Copy,Clone,Default)]
pub struct LiteralBlockSwitch(pub BlockSwitch, pub u8);

impl LiteralBlockSwitch {
    pub fn new(block_type: u8, stride: u8) -> Self {
        LiteralBlockSwitch(BlockSwitch::new(block_type), stride)
    }
    pub fn block_type(&self) -> u8 {
        self.0.block_type()
    }
    pub fn stride(&self) -> u8 {
        self.1
    }
    pub fn update_stride(&mut self, new_stride: u8) {
        self.1 = new_stride;
    }
}

pub const LITERAL_PREDICTION_MODE_SIGN: u8 = 3;
pub const LITERAL_PREDICTION_MODE_UTF8: u8 = 2;
pub const LITERAL_PREDICTION_MODE_MSB6: u8 = 1;
pub const LITERAL_PREDICTION_MODE_LSB6: u8 = 0;

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct LiteralPredictionModeNibble(pub u8);

impl LiteralPredictionModeNibble {
    pub fn new(prediction_mode: u8) -> Result<Self, ()> {
        if prediction_mode < 16 {
            return Ok(LiteralPredictionModeNibble(prediction_mode));
        }
        return Err(());
    }
    pub fn prediction_mode(&self) -> u8 {
        self.0
    }
    pub fn signed() -> Self {
        LiteralPredictionModeNibble(LITERAL_PREDICTION_MODE_SIGN)
    }
    pub fn utf8() -> Self {
        LiteralPredictionModeNibble(LITERAL_PREDICTION_MODE_UTF8)
    }
    pub fn msb6() -> Self {
        LiteralPredictionModeNibble(LITERAL_PREDICTION_MODE_MSB6)
    }
    pub fn lsb6() -> Self {
        LiteralPredictionModeNibble(LITERAL_PREDICTION_MODE_LSB6)
    }
}
#[derive(Debug)]
pub struct PredictionModeContextMap<SliceType:SliceWrapper<u8>> {
    pub literal_prediction_mode: LiteralPredictionModeNibble,
    pub literal_context_map: SliceType,
    pub distance_context_map: SliceType,
}
impl<SliceType:SliceWrapper<u8>+Clone> Clone for PredictionModeContextMap<SliceType> {
    fn clone(&self) -> PredictionModeContextMap<SliceType>{
        PredictionModeContextMap::<SliceType>{
            literal_prediction_mode:self.literal_prediction_mode,
            literal_context_map:self.literal_context_map.clone(),
            distance_context_map:self.distance_context_map.clone(),
        }
    }
}

impl<SliceType:SliceWrapper<u8>+Clone+Copy> Copy for PredictionModeContextMap<SliceType> {
}


#[derive(Debug,Clone,Copy)]
pub struct CopyCommand {
    pub distance: u32,
    pub num_bytes: u32,
}

impl Nop<CopyCommand> for CopyCommand {
    fn nop() -> Self {
        CopyCommand {
            distance: 1,
            num_bytes: 0
        }
    }
}

#[derive(Debug,Clone,Copy)]
pub struct DictCommand {
    pub word_size: u8,
    pub transform: u8,
    pub final_size: u8,
    pub empty: u8,
    pub word_id: u32,
}

impl Nop<DictCommand> for DictCommand {
    fn nop() -> Self {
        DictCommand {
            word_size: 0,
            transform: 0,
            final_size: 0,
            empty: 1,
            word_id: 0
        }
    }
}

#[derive(Debug)]
#[cfg(not(feature="external-literal-probability"))]
pub struct FeatureFlagSliceType<SliceType:SliceWrapper<u8> >(core::marker::PhantomData<*const SliceType>);

#[cfg(not(feature="external-literal-probability"))]
impl<SliceType:SliceWrapper<u8>> SliceWrapper<u8> for FeatureFlagSliceType<SliceType> {
   fn slice(&self) -> &[u8] {
       &[]
   }
}

#[cfg(not(feature="external-literal-probability"))]
impl<SliceType:SliceWrapper<u8>+Default> Default for FeatureFlagSliceType<SliceType> {
    fn default() -> Self {
        FeatureFlagSliceType::<SliceType>(core::marker::PhantomData::<*const SliceType>::default())
    }
}



#[derive(Debug)]
#[cfg(feature="external-literal-probability")]
pub struct FeatureFlagSliceType<SliceType:SliceWrapper<u8> >(pub SliceType);

#[cfg(feature="external-literal-probability")]
impl<SliceType:SliceWrapper<u8>> SliceWrapper<u8> for FeatureFlagSliceType<SliceType> {
   fn slice(&self) -> &[u8] {
       self.0.slice()
   }
}

#[cfg(feature="external-literal-probability")]
impl<SliceType:SliceWrapper<u8>+Default> Default for FeatureFlagSliceType<SliceType> {
    fn default() -> Self {
        FeatureFlagSliceType::<SliceType>(SliceType::default())
    }
}



impl<SliceType:SliceWrapper<u8>+Clone> Clone for FeatureFlagSliceType<SliceType> {
    fn clone(&self) -> Self {
       FeatureFlagSliceType::<SliceType>(self.0.clone())
    }
}
impl<SliceType:SliceWrapper<u8>+Clone+Copy> Copy for FeatureFlagSliceType<SliceType> {
}


#[derive(Debug)]
pub struct LiteralCommand<SliceType:SliceWrapper<u8>> {
    pub data: SliceType,
    pub prob: FeatureFlagSliceType<SliceType>
}

impl<SliceType:SliceWrapper<u8>+Default> Nop<LiteralCommand<SliceType>> for LiteralCommand<SliceType> {
    fn nop() -> Self {
        LiteralCommand {
            data: SliceType::default(),
            prob: FeatureFlagSliceType::<SliceType>::default(),
        }
    }
}
impl<SliceType:SliceWrapper<u8>+Clone> Clone for LiteralCommand<SliceType> {
    fn clone(&self) -> LiteralCommand<SliceType>{
        LiteralCommand::<SliceType>{data:self.data.clone(), prob:self.prob.clone()}
    }
}
impl<SliceType:SliceWrapper<u8>+Clone+Copy> Copy for LiteralCommand<SliceType> {
}

#[derive(Debug)]
pub enum Command<SliceType:SliceWrapper<u8> > {
    Copy(CopyCommand),
    Dict(DictCommand),
    Literal(LiteralCommand<SliceType>),
    BlockSwitchCommand(BlockSwitch),
    BlockSwitchLiteral(LiteralBlockSwitch),
    BlockSwitchDistance(BlockSwitch),
    PredictionMode(PredictionModeContextMap<SliceType>),
}
impl<SliceType:SliceWrapper<u8>+Default> Command<SliceType> {
    pub fn free_array<F>(&mut self, apply_func: &mut F) where F: FnMut(SliceType) {
       match self {
          &mut Command::Literal(ref mut lit) => {
             apply_func(core::mem::replace(&mut lit.data, SliceType::default()))
          },
          &mut Command::PredictionMode(ref mut pm) => {
             apply_func(core::mem::replace(&mut pm.literal_context_map, SliceType::default()));
             apply_func(core::mem::replace(&mut pm.distance_context_map, SliceType::default()));
          },
          _ => {},
       }
    }
}


impl<SliceType:SliceWrapper<u8>> Default for Command<SliceType> {
    fn default() -> Self {
        Command::<SliceType>::nop()
    }
}

impl<SliceType:SliceWrapper<u8>> Nop<Command<SliceType>> for Command<SliceType> {
    fn nop() -> Command<SliceType> {
        Command::Copy(CopyCommand::nop())
    }
}

impl<SliceType:SliceWrapper<u8>+Clone> Clone for Command<SliceType> {
    fn clone(&self) -> Command<SliceType>{
        match self {
            &Command::Copy(ref copy) => Command::Copy(copy.clone()),
            &Command::Dict(ref dict) => Command::Dict(dict.clone()),
            &Command::Literal(ref literal) => Command::Literal(literal.clone()),
            &Command::BlockSwitchCommand(ref switch) => Command::BlockSwitchCommand(switch.clone()),
            &Command::BlockSwitchLiteral(ref switch) => Command::BlockSwitchLiteral(switch.clone()),
            &Command::BlockSwitchDistance(ref switch) => Command::BlockSwitchDistance(switch.clone()),
            &Command::PredictionMode(ref pm) => Command::PredictionMode(pm.clone()),
        }
    }
}

impl<SliceType:SliceWrapper<u8>+Clone+Copy> Copy for Command<SliceType> {
}

pub fn free_cmd<SliceTypeAllocator:Allocator<u8>> (xself: &mut Command<SliceTypeAllocator::AllocatedMemory>, m8: &mut SliceTypeAllocator) {
       match xself {
          &mut Command::Literal(ref mut lit) => {
             m8.free_cell(core::mem::replace(&mut lit.data, SliceTypeAllocator::AllocatedMemory::default()))
          },
          &mut Command::PredictionMode(ref mut pm) => {
             m8.free_cell(core::mem::replace(&mut pm.literal_context_map, SliceTypeAllocator::AllocatedMemory::default()));
             m8.free_cell(core::mem::replace(&mut pm.distance_context_map, SliceTypeAllocator::AllocatedMemory::default()));
          },
          _ => {},
    }
}
