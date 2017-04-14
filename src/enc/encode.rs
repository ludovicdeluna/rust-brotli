use super::compress_fragment_two_pass::BrotliCompressFragmentTwoPass;
use super::compress_fragment::BrotliCompressFragmentFast;

use super::metablock::{BrotliBuildMetaBlock, BrotliBuildMetaBlockGreedy, BrotliOptimizeHistograms};
use super::backward_references::{BrotliCreateBackwardReferences, Struct1};
use super::block_split::{BlockSplit};
use super::utf8_util::{BrotliIsMostlyUTF8};
use super::command::{Command};
use core;

use super::bit_cost::BitsEntropy;
use super::brotli_bit_stream::{BrotliBuildAndStoreHuffmanTreeFast, BrotliStoreHuffmanTree,
                               BrotliStoreMetaBlock, BrotliStoreMetaBlockFast, BrotliStoreMetaBlockTrivial,
                               BrotliStoreUncompressedMetaBlock,
};
use super::entropy_encode::{BrotliConvertBitDepthsToSymbols, BrotliCreateHuffmanTree, HuffmanTree,
                            NewHuffmanTree};
use super::static_dict::{BROTLI_UNALIGNED_LOAD32, BROTLI_UNALIGNED_LOAD64, BROTLI_UNALIGNED_STORE64,
                         FindMatchLengthWithLimit, BrotliGetDictionary};
use super::super::alloc;
use super::super::alloc::{SliceWrapper, SliceWrapperMut};
use super::util::{brotli_min_size_t, Log2FloorNonZero};

  //fn BrotliCreateHqZopfliBackwardReferences(m: &mut [MemoryManager],
  //                                          dictionary: &[BrotliDictionary],
  //                                          num_bytes: usize,
  //                                          position: usize,
  //                                          ringbuffer: &[u8],
  //                                          ringbuffer_mask: usize,
  //                                          params: &[BrotliEncoderParams],
  //                                          hasher: &mut [u8],
  //                                          dist_cache: &mut [i32],
  //                                          last_insert_len: &mut [usize],
  //                                          commands: &mut [Command],
  //                                          num_commands: &mut [usize],
  //                                          num_literals: &mut [usize]);
  //fn BrotliCreateZopfliBackwardReferences(m: &mut [MemoryManager],
   //                                       dictionary: &[BrotliDictionary],
    //                                      num_bytes: usize,
  //                                        position: usize,
  //                                        ringbuffer: &[u8],
  //                                        ringbuffer_mask: usize,
  //                                        params: &[BrotliEncoderParams],
  //                                        hasher: &mut [u8],
  //                                        dist_cache: &mut [i32],
  //                                        last_insert_len: &mut [usize],
  //                                        commands: &mut [Command],
  //                                        num_commands: &mut [usize],
  //                                        num_literals: &mut [usize]);
  //fn BrotliInitBlockSplit(xself: &mut BlockSplit);
  //fn BrotliInitMemoryManager(m: &mut [MemoryManager],
  //                           alloc_func: fn(&mut [::std::os::raw::c_void], usize)
  //                                          -> *mut ::std::os::raw::c_void,
  //                           free_func: fn(*mut ::std::os::raw::c_void,
  //                                         *mut ::std::os::raw::c_void),
  //                           opaque: *mut ::std::os::raw::c_void);
  //fn BrotliInitZopfliNodes(array: &mut [ZopfliNode], length: usize);
  //fn BrotliWipeOutMemoryManager(m: &mut [MemoryManager]);


static kBrotliMinWindowBits: i32 = 10i32;

static kBrotliMaxWindowBits: i32 = 24i32;

static kInvalidMatch: u32 = 0xfffffffu32;

static kCutoffTransformsCount: u32 = 10u32;

static kCutoffTransforms: usize = 0x71b520ausize << 32i32 | 0xda2d3200u32 as (usize);

static kHashMul32: u32 = 0x1e35a7bdu32;

static kHashMul64: usize = 0x1e35a7bdusize << 32i32 | 0x1e35a7bdusize;

static kHashMul64Long: usize = 0x1fe35a7bu32 as (usize) << 32i32 | 0xd3579bd3u32 as (usize);


static kCompressFragmentTwoPassBlockSize: usize = (1i32 << 17i32) as (usize);

static kMinUTF8Ratio: f64 = 0.75f64;

#[repr(i32)]
pub enum BrotliEncoderParameter {
  BROTLI_PARAM_MODE = 0i32,
  BROTLI_PARAM_QUALITY = 1i32,
  BROTLI_PARAM_LGWIN = 2i32,
  BROTLI_PARAM_LGBLOCK = 3i32,
  BROTLI_PARAM_DISABLE_LITERAL_CONTEXT_MODELING = 4i32,
  BROTLI_PARAM_SIZE_HINT = 5i32,
}


#[repr(i32)]
pub enum BrotliEncoderMode {
  BROTLI_MODE_GENERIC = 0i32,
  BROTLI_MODE_TEXT = 1i32,
  BROTLI_MODE_FONT = 2i32,
}





pub struct RingBuffer {
  pub size_: u32,
  pub mask_: u32,
  pub tail_size_: u32,
  pub total_size_: u32,
  pub cur_size_: u32,
  pub pos_: u32,
  pub data_: *mut u8,
  pub buffer_index: usize,
}


#[repr(i32)]
pub enum BrotliEncoderStreamState {
  BROTLI_STREAM_PROCESSING = 0i32,
  BROTLI_STREAM_FLUSH_REQUESTED = 1i32,
  BROTLI_STREAM_FINISHED = 2i32,
  BROTLI_STREAM_METADATA_HEAD = 3i32,
  BROTLI_STREAM_METADATA_BODY = 4i32,
}

/*

pub struct BrotliEncoderStateStruct {
  pub params: BrotliEncoderParams,
  pub memory_manager_: MemoryManager,
  pub hasher_: *mut u8,
  pub input_pos_: usize,
  pub ringbuffer_: RingBuffer,
  pub cmd_alloc_size_: usize,
  pub commands_: *mut Command,
  pub num_commands_: usize,
  pub num_literals_: usize,
  pub last_insert_len_: usize,
  pub last_flush_pos_: usize,
  pub last_processed_pos_: usize,
  pub dist_cache_: [i32; 16],
  pub saved_dist_cache_: [i32; 4],
  pub last_byte_: u8,
  pub last_byte_bits_: u8,
  pub prev_byte_: u8,
  pub prev_byte2_: u8,
  pub storage_size_: usize,
  pub storage_: *mut u8,
  pub small_table_: [i32; 1024],
  pub large_table_: *mut i32,
  pub large_table_size_: usize,
  pub cmd_depths_: [u8; 128],
  pub cmd_bits_: [u16; 128],
  pub cmd_code_: [u8; 512],
  pub cmd_code_numbits_: usize,
  pub command_buf_: *mut u32,
  pub literal_buf_: *mut u8,
  pub next_out_: *mut u8,
  pub available_out_: usize,
  pub total_out_: usize,
  pub tiny_buf_: Struct1,
  pub remaining_metadata_bytes_: u32,
  pub stream_state_: BrotliEncoderStreamState,
  pub is_last_block_emitted_: i32,
  pub is_initialized_: i32,
}


pub fn BrotliEncoderSetParameter(mut state: &mut [BrotliEncoderStateStruct],
                                 mut p: BrotliEncoderParameter,
                                 mut value: u32)
                                 -> i32 {
  if (*state).is_initialized_ != 0 {
    return 0i32;
  }
  if p as (i32) == BrotliEncoderParameter::BROTLI_PARAM_MODE as (i32) {
    (*state).params.mode = value as (BrotliEncoderMode);
    return 1i32;
  }
  if p as (i32) == BrotliEncoderParameter::BROTLI_PARAM_QUALITY as (i32) {
    (*state).params.quality = value as (i32);
    return 1i32;
  }
  if p as (i32) == BrotliEncoderParameter::BROTLI_PARAM_LGWIN as (i32) {
    (*state).params.lgwin = value as (i32);
    return 1i32;
  }
  if p as (i32) == BrotliEncoderParameter::BROTLI_PARAM_LGBLOCK as (i32) {
    (*state).params.lgblock = value as (i32);
    return 1i32;
  }
  if p as (i32) == BrotliEncoderParameter::BROTLI_PARAM_DISABLE_LITERAL_CONTEXT_MODELING as (i32) {
    if value != 0u32 && (value != 1u32) {
      return 0i32;
    }
    (*state).params.disable_literal_context_modeling = if !!!(value == 0) { 1i32 } else { 0i32 };
    return 1i32;
  }
  if p as (i32) == BrotliEncoderParameter::BROTLI_PARAM_SIZE_HINT as (i32) {
    (*state).params.size_hint = value as (usize);
    return 1i32;
  }
  0i32
}

fn BrotliEncoderInitParams(mut params: &mut [BrotliEncoderParams]) {
  (*params).mode = BrotliEncoderMode::BROTLI_MODE_GENERIC;
  (*params).quality = 11i32;
  (*params).lgwin = 22i32;
  (*params).lgblock = 0i32;
  (*params).size_hint = 0usize;
  (*params).disable_literal_context_modeling = 0i32;
}

fn RingBufferInit(mut rb: &mut [RingBuffer]) {
  (*rb).cur_size_ = 0u32;
  (*rb).pos_ = 0u32;
  (*rb).data_ = 0i32;
  (*rb).buffer_index = 0usize;
}

fn BrotliEncoderInitState(mut s: &mut [BrotliEncoderStateStruct]) {
  BrotliEncoderInitParams(&mut (*s).params);
  (*s).input_pos_ = 0usize;
  (*s).num_commands_ = 0usize;
  (*s).num_literals_ = 0usize;
  (*s).last_insert_len_ = 0usize;
  (*s).last_flush_pos_ = 0usize;
  (*s).last_processed_pos_ = 0usize;
  (*s).prev_byte_ = 0i32 as (u8);
  (*s).prev_byte2_ = 0i32 as (u8);
  (*s).storage_size_ = 0usize;
  (*s).storage_ = 0i32;
  (*s).hasher_ = 0i32;
  (*s).large_table_ = 0i32;
  (*s).large_table_size_ = 0usize;
  (*s).cmd_code_numbits_ = 0usize;
  (*s).command_buf_ = 0i32;
  (*s).literal_buf_ = 0i32;
  (*s).next_out_ = 0i32;
  (*s).available_out_ = 0usize;
  (*s).total_out_ = 0usize;
  (*s).stream_state_ = BrotliEncoderStreamState::BROTLI_STREAM_PROCESSING;
  (*s).is_last_block_emitted_ = 0i32;
  (*s).is_initialized_ = 0i32;
  RingBufferInit(&mut (*s).ringbuffer_);
  (*s).commands_ = 0i32;
  (*s).cmd_alloc_size_ = 0usize;
  (*s).dist_cache_[0usize] = 4i32;
  (*s).dist_cache_[1usize] = 11i32;
  (*s).dist_cache_[2usize] = 15i32;
  (*s).dist_cache_[3usize] = 16i32;
  memcpy((*s).saved_dist_cache_.as_mut_ptr(),
         (*s).dist_cache_.as_mut_ptr(),
         ::std::mem::size_of::<[i32; 4]>());
}


pub fn BrotliEncoderCreateInstance(mut alloc_func: fn(&mut [::std::os::raw::c_void], usize)
                                                      -> *mut ::std::os::raw::c_void,
                                   mut free_func: fn(*mut ::std::os::raw::c_void,
                                                     *mut ::std::os::raw::c_void),
                                   mut opaque: *mut ::std::os::raw::c_void)
                                   -> *mut BrotliEncoderStateStruct {
  let mut state: *mut BrotliEncoderStateStruct = 0i32;
  if alloc_func == 0 && (free_func == 0) {
    state = malloc(::std::mem::size_of::<BrotliEncoderStateStruct>());
  } else if alloc_func != 0 && (free_func != 0) {
    state = alloc_func(opaque, ::std::mem::size_of::<BrotliEncoderStateStruct>());
  }
  if state == 0i32 {
    return 0i32;
  }
  BrotliInitMemoryManager(&mut (*state).memory_manager_, alloc_func, free_func, opaque);
  BrotliEncoderInitState(state);
  state
}

fn RingBufferFree(mut m: &mut [MemoryManager], mut rb: &mut [RingBuffer]) {
  BrotliFree(m, (*rb).data_);
  (*rb).data_ = 0i32;
}

fn DestroyHasher(mut m: &mut [MemoryManager], mut handle: &mut [*mut u8]) {
  if *handle == 0i32 {
    return;
  }
  {
    BrotliFree(m, *handle);
    *handle = 0i32;
  }
}

fn BrotliEncoderCleanupState(mut s: &mut [BrotliEncoderStateStruct]) {
  let mut m: *mut MemoryManager = &mut (*s).memory_manager_;
  if !(0i32 == 0) {
    BrotliWipeOutMemoryManager(m);
    return;
  }
  {
    BrotliFree(m, (*s).storage_);
    (*s).storage_ = 0i32;
  }
  {
    BrotliFree(m, (*s).commands_);
    (*s).commands_ = 0i32;
  }
  RingBufferFree(m, &mut (*s).ringbuffer_);
  DestroyHasher(m, &mut (*s).hasher_);
  {
    BrotliFree(m, (*s).large_table_);
    (*s).large_table_ = 0i32;
  }
  {
    BrotliFree(m, (*s).command_buf_);
    (*s).command_buf_ = 0i32;
  }
  {
    BrotliFree(m, (*s).literal_buf_);
    (*s).literal_buf_ = 0i32;
  }
}


pub fn BrotliEncoderDestroyInstance(mut state: &mut [BrotliEncoderStateStruct]) {
  if state.is_null() {
  } else {
    let mut m: *mut MemoryManager = &mut (*state).memory_manager_;
    let mut free_func: fn(*mut ::std::os::raw::c_void, *mut ::std::os::raw::c_void) =
      (*m).free_func;
    let mut opaque: *mut ::std::os::raw::c_void = (*m).opaque;
    BrotliEncoderCleanupState(state);
    free_func(opaque, state);
  }
}

fn brotli_min_int(mut a: i32, mut b: i32) -> i32 {
  if a < b { a } else { b }
}

fn brotli_max_int(mut a: i32, mut b: i32) -> i32 {
  if a > b { a } else { b }
}

fn SanitizeParams(mut params: &mut [BrotliEncoderParams]) {
  (*params).quality = brotli_min_int(11i32, brotli_max_int(0i32, (*params).quality));
  if (*params).lgwin < 10i32 {
    (*params).lgwin = 10i32;
  } else if (*params).lgwin > 24i32 {
    (*params).lgwin = 24i32;
  }
}

fn ComputeLgBlock(mut params: &[BrotliEncoderParams]) -> i32 {
  let mut lgblock: i32 = (*params).lgblock;
  if (*params).quality == 0i32 || (*params).quality == 1i32 {
    lgblock = (*params).lgwin;
  } else if (*params).quality < 4i32 {
    lgblock = 14i32;
  } else if lgblock == 0i32 {
    lgblock = 16i32;
    if (*params).quality >= 9i32 && ((*params).lgwin > lgblock) {
      lgblock = brotli_min_int(18i32, (*params).lgwin);
    }
  } else {
    lgblock = brotli_min_int(24i32, brotli_max_int(16i32, lgblock));
  }
  lgblock
}

fn ComputeRbBits(mut params: &[BrotliEncoderParams]) -> i32 {
  1i32 + brotli_max_int((*params).lgwin, (*params).lgblock)
}

fn RingBufferSetup(mut params: &[BrotliEncoderParams], mut rb: &mut [RingBuffer]) {
  let mut window_bits: i32 = ComputeRbBits(params);
  let mut tail_bits: i32 = (*params).lgblock;
  *(&mut (*rb).size_) = 1u32 << window_bits;
  *(&mut (*rb).mask_) = (1u32 << window_bits).wrapping_sub(1u32);
  *(&mut (*rb).tail_size_) = 1u32 << tail_bits;
  *(&mut (*rb).total_size_) = (*rb).size_.wrapping_add((*rb).tail_size_);
}

fn EncodeWindowBits(mut lgwin: i32, mut last_byte: &mut [u8], mut last_byte_bits: &mut [u8]) {
  if lgwin == 16i32 {
    *last_byte = 0i32 as (u8);
    *last_byte_bits = 1i32 as (u8);
  } else if lgwin == 17i32 {
    *last_byte = 1i32 as (u8);
    *last_byte_bits = 7i32 as (u8);
  } else if lgwin > 17i32 {
    *last_byte = (lgwin - 17i32 << 1i32 | 1i32) as (u8);
    *last_byte_bits = 4i32 as (u8);
  } else {
    *last_byte = (lgwin - 8i32 << 4i32 | 1i32) as (u8);
    *last_byte_bits = 7i32 as (u8);
  }
}

fn InitCommandPrefixCodes(mut cmd_depths: &mut [u8],
                          mut cmd_bits: &mut [u16],
                          mut cmd_code: &mut [u8],
                          mut cmd_code_numbits: &mut [usize]) {
  static mut kDefaultCommandDepths: [u8; 128] = [0i32 as (u8),
                                                 4i32 as (u8),
                                                 4i32 as (u8),
                                                 5i32 as (u8),
                                                 6i32 as (u8),
                                                 6i32 as (u8),
                                                 7i32 as (u8),
                                                 7i32 as (u8),
                                                 7i32 as (u8),
                                                 7i32 as (u8),
                                                 7i32 as (u8),
                                                 8i32 as (u8),
                                                 8i32 as (u8),
                                                 8i32 as (u8),
                                                 8i32 as (u8),
                                                 8i32 as (u8),
                                                 0i32 as (u8),
                                                 0i32 as (u8),
                                                 0i32 as (u8),
                                                 4i32 as (u8),
                                                 4i32 as (u8),
                                                 4i32 as (u8),
                                                 4i32 as (u8),
                                                 4i32 as (u8),
                                                 5i32 as (u8),
                                                 5i32 as (u8),
                                                 6i32 as (u8),
                                                 6i32 as (u8),
                                                 6i32 as (u8),
                                                 6i32 as (u8),
                                                 7i32 as (u8),
                                                 7i32 as (u8),
                                                 7i32 as (u8),
                                                 7i32 as (u8),
                                                 10i32 as (u8),
                                                 10i32 as (u8),
                                                 10i32 as (u8),
                                                 10i32 as (u8),
                                                 10i32 as (u8),
                                                 10i32 as (u8),
                                                 0i32 as (u8),
                                                 4i32 as (u8),
                                                 4i32 as (u8),
                                                 5i32 as (u8),
                                                 5i32 as (u8),
                                                 5i32 as (u8),
                                                 6i32 as (u8),
                                                 6i32 as (u8),
                                                 7i32 as (u8),
                                                 8i32 as (u8),
                                                 8i32 as (u8),
                                                 9i32 as (u8),
                                                 10i32 as (u8),
                                                 10i32 as (u8),
                                                 10i32 as (u8),
                                                 10i32 as (u8),
                                                 10i32 as (u8),
                                                 10i32 as (u8),
                                                 10i32 as (u8),
                                                 10i32 as (u8),
                                                 10i32 as (u8),
                                                 10i32 as (u8),
                                                 10i32 as (u8),
                                                 10i32 as (u8),
                                                 5i32 as (u8),
                                                 0i32 as (u8),
                                                 0i32 as (u8),
                                                 0i32 as (u8),
                                                 0i32 as (u8),
                                                 0i32 as (u8),
                                                 0i32 as (u8),
                                                 0i32 as (u8),
                                                 0i32 as (u8),
                                                 0i32 as (u8),
                                                 0i32 as (u8),
                                                 0i32 as (u8),
                                                 0i32 as (u8),
                                                 0i32 as (u8),
                                                 0i32 as (u8),
                                                 0i32 as (u8),
                                                 6i32 as (u8),
                                                 6i32 as (u8),
                                                 6i32 as (u8),
                                                 6i32 as (u8),
                                                 6i32 as (u8),
                                                 6i32 as (u8),
                                                 5i32 as (u8),
                                                 5i32 as (u8),
                                                 5i32 as (u8),
                                                 5i32 as (u8),
                                                 5i32 as (u8),
                                                 5i32 as (u8),
                                                 4i32 as (u8),
                                                 4i32 as (u8),
                                                 4i32 as (u8),
                                                 4i32 as (u8),
                                                 4i32 as (u8),
                                                 4i32 as (u8),
                                                 4i32 as (u8),
                                                 5i32 as (u8),
                                                 5i32 as (u8),
                                                 5i32 as (u8),
                                                 5i32 as (u8),
                                                 5i32 as (u8),
                                                 5i32 as (u8),
                                                 6i32 as (u8),
                                                 6i32 as (u8),
                                                 7i32 as (u8),
                                                 7i32 as (u8),
                                                 7i32 as (u8),
                                                 8i32 as (u8),
                                                 10i32 as (u8),
                                                 12i32 as (u8),
                                                 12i32 as (u8),
                                                 12i32 as (u8),
                                                 12i32 as (u8),
                                                 12i32 as (u8),
                                                 12i32 as (u8),
                                                 12i32 as (u8),
                                                 12i32 as (u8),
                                                 12i32 as (u8),
                                                 12i32 as (u8),
                                                 12i32 as (u8),
                                                 12i32 as (u8),
                                                 0i32 as (u8),
                                                 0i32 as (u8),
                                                 0i32 as (u8),
                                                 0i32 as (u8)];
  static mut kDefaultCommandBits: [u16; 128] = [0i32 as (u16),
                                                0i32 as (u16),
                                                8i32 as (u16),
                                                9i32 as (u16),
                                                3i32 as (u16),
                                                35i32 as (u16),
                                                7i32 as (u16),
                                                71i32 as (u16),
                                                39i32 as (u16),
                                                103i32 as (u16),
                                                23i32 as (u16),
                                                47i32 as (u16),
                                                175i32 as (u16),
                                                111i32 as (u16),
                                                239i32 as (u16),
                                                31i32 as (u16),
                                                0i32 as (u16),
                                                0i32 as (u16),
                                                0i32 as (u16),
                                                4i32 as (u16),
                                                12i32 as (u16),
                                                2i32 as (u16),
                                                10i32 as (u16),
                                                6i32 as (u16),
                                                13i32 as (u16),
                                                29i32 as (u16),
                                                11i32 as (u16),
                                                43i32 as (u16),
                                                27i32 as (u16),
                                                59i32 as (u16),
                                                87i32 as (u16),
                                                55i32 as (u16),
                                                15i32 as (u16),
                                                79i32 as (u16),
                                                319i32 as (u16),
                                                831i32 as (u16),
                                                191i32 as (u16),
                                                703i32 as (u16),
                                                447i32 as (u16),
                                                959i32 as (u16),
                                                0i32 as (u16),
                                                14i32 as (u16),
                                                1i32 as (u16),
                                                25i32 as (u16),
                                                5i32 as (u16),
                                                21i32 as (u16),
                                                19i32 as (u16),
                                                51i32 as (u16),
                                                119i32 as (u16),
                                                159i32 as (u16),
                                                95i32 as (u16),
                                                223i32 as (u16),
                                                479i32 as (u16),
                                                991i32 as (u16),
                                                63i32 as (u16),
                                                575i32 as (u16),
                                                127i32 as (u16),
                                                639i32 as (u16),
                                                383i32 as (u16),
                                                895i32 as (u16),
                                                255i32 as (u16),
                                                767i32 as (u16),
                                                511i32 as (u16),
                                                1023i32 as (u16),
                                                14i32 as (u16),
                                                0i32 as (u16),
                                                0i32 as (u16),
                                                0i32 as (u16),
                                                0i32 as (u16),
                                                0i32 as (u16),
                                                0i32 as (u16),
                                                0i32 as (u16),
                                                0i32 as (u16),
                                                0i32 as (u16),
                                                0i32 as (u16),
                                                0i32 as (u16),
                                                0i32 as (u16),
                                                0i32 as (u16),
                                                0i32 as (u16),
                                                0i32 as (u16),
                                                27i32 as (u16),
                                                59i32 as (u16),
                                                7i32 as (u16),
                                                39i32 as (u16),
                                                23i32 as (u16),
                                                55i32 as (u16),
                                                30i32 as (u16),
                                                1i32 as (u16),
                                                17i32 as (u16),
                                                9i32 as (u16),
                                                25i32 as (u16),
                                                5i32 as (u16),
                                                0i32 as (u16),
                                                8i32 as (u16),
                                                4i32 as (u16),
                                                12i32 as (u16),
                                                2i32 as (u16),
                                                10i32 as (u16),
                                                6i32 as (u16),
                                                21i32 as (u16),
                                                13i32 as (u16),
                                                29i32 as (u16),
                                                3i32 as (u16),
                                                19i32 as (u16),
                                                11i32 as (u16),
                                                15i32 as (u16),
                                                47i32 as (u16),
                                                31i32 as (u16),
                                                95i32 as (u16),
                                                63i32 as (u16),
                                                127i32 as (u16),
                                                255i32 as (u16),
                                                767i32 as (u16),
                                                2815i32 as (u16),
                                                1791i32 as (u16),
                                                3839i32 as (u16),
                                                511i32 as (u16),
                                                2559i32 as (u16),
                                                1535i32 as (u16),
                                                3583i32 as (u16),
                                                1023i32 as (u16),
                                                3071i32 as (u16),
                                                2047i32 as (u16),
                                                4095i32 as (u16),
                                                0i32 as (u16),
                                                0i32 as (u16),
                                                0i32 as (u16),
                                                0i32 as (u16)];
  static mut kDefaultCommandCode: [u8; 57] = [0xffi32 as (u8),
                                              0x77i32 as (u8),
                                              0xd5i32 as (u8),
                                              0xbfi32 as (u8),
                                              0xe7i32 as (u8),
                                              0xdei32 as (u8),
                                              0xeai32 as (u8),
                                              0x9ei32 as (u8),
                                              0x51i32 as (u8),
                                              0x5di32 as (u8),
                                              0xdei32 as (u8),
                                              0xc6i32 as (u8),
                                              0x70i32 as (u8),
                                              0x57i32 as (u8),
                                              0xbci32 as (u8),
                                              0x58i32 as (u8),
                                              0x58i32 as (u8),
                                              0x58i32 as (u8),
                                              0xd8i32 as (u8),
                                              0xd8i32 as (u8),
                                              0x58i32 as (u8),
                                              0xd5i32 as (u8),
                                              0xcbi32 as (u8),
                                              0x8ci32 as (u8),
                                              0xeai32 as (u8),
                                              0xe0i32 as (u8),
                                              0xc3i32 as (u8),
                                              0x87i32 as (u8),
                                              0x1fi32 as (u8),
                                              0x83i32 as (u8),
                                              0xc1i32 as (u8),
                                              0x60i32 as (u8),
                                              0x1ci32 as (u8),
                                              0x67i32 as (u8),
                                              0xb2i32 as (u8),
                                              0xaai32 as (u8),
                                              0x6i32 as (u8),
                                              0x83i32 as (u8),
                                              0xc1i32 as (u8),
                                              0x60i32 as (u8),
                                              0x30i32 as (u8),
                                              0x18i32 as (u8),
                                              0xcci32 as (u8),
                                              0xa1i32 as (u8),
                                              0xcei32 as (u8),
                                              0x88i32 as (u8),
                                              0x54i32 as (u8),
                                              0x94i32 as (u8),
                                              0x46i32 as (u8),
                                              0xe1i32 as (u8),
                                              0xb0i32 as (u8),
                                              0xd0i32 as (u8),
                                              0x4ei32 as (u8),
                                              0xb2i32 as (u8),
                                              0xf7i32 as (u8),
                                              0x4i32 as (u8),
                                              0x0i32 as (u8)];
  static kDefaultCommandCodeNumBits: usize = 448usize;
  memcpy(cmd_depths,
         kDefaultCommandDepths.as_ptr(),
         ::std::mem::size_of::<[u8; 128]>());
  memcpy(cmd_bits,
         kDefaultCommandBits.as_ptr(),
         ::std::mem::size_of::<[u16; 128]>());
  memcpy(cmd_code,
         kDefaultCommandCode.as_ptr(),
         ::std::mem::size_of::<[u8; 57]>());
  *cmd_code_numbits = kDefaultCommandCodeNumBits;
}

fn EnsureInitialized(mut s: &mut [BrotliEncoderStateStruct]) -> i32 {
  if !(0i32 == 0) {
    return 0i32;
  }
  if (*s).is_initialized_ != 0 {
    return 1i32;
  }
  SanitizeParams(&mut (*s).params);
  (*s).params.lgblock = ComputeLgBlock(&mut (*s).params);
  (*s).remaining_metadata_bytes_ = !(0u32);
  RingBufferSetup(&mut (*s).params, &mut (*s).ringbuffer_);
  {
    let mut lgwin: i32 = (*s).params.lgwin;
    if (*s).params.quality == 0i32 || (*s).params.quality == 1i32 {
      lgwin = brotli_max_int(lgwin, 18i32);
    }
    EncodeWindowBits(lgwin, &mut (*s).last_byte_, &mut (*s).last_byte_bits_);
  }
  if (*s).params.quality == 0i32 {
    InitCommandPrefixCodes((*s).cmd_depths_.as_mut_ptr(),
                           (*s).cmd_bits_.as_mut_ptr(),
                           (*s).cmd_code_.as_mut_ptr(),
                           &mut (*s).cmd_code_numbits_);
  }
  (*s).is_initialized_ = 1i32;
  1i32
}

fn RingBufferInitBuffer(mut m: &mut [MemoryManager], buflen: u32, mut rb: &mut [RingBuffer]) {
  static kSlackForEightByteHashingEverywhere: usize = 7usize;
  let mut new_data: *mut u8 = if ((2u32).wrapping_add(buflen) as (usize))
       .wrapping_add(kSlackForEightByteHashingEverywhere) != 0 {
    BrotliAllocate(m,
                   ((2u32).wrapping_add(buflen) as (usize))
                     .wrapping_add(kSlackForEightByteHashingEverywhere)
                     .wrapping_mul(::std::mem::size_of::<u8>()))
  } else {
    0i32
  };
  let mut i: usize;
  if !(0i32 == 0) {
    return;
  }
  if !(*rb).data_.is_null() {
    memcpy(new_data,
           (*rb).data_,
           ((2u32).wrapping_add((*rb).cur_size_) as (usize))
             .wrapping_add(kSlackForEightByteHashingEverywhere));
    {
      BrotliFree(m, (*rb).data_);
      (*rb).data_ = 0i32;
    }
  }
  (*rb).data_ = new_data;
  (*rb).cur_size_ = buflen;
  (*rb).buffer_index = 2usize;
  *(*rb).data_[((*rb).buffer_index.wrapping_sub(2usize) as (usize))..] = {
    let _rhs = 0i32;
    let _lhs = &mut *(*rb).data_[((*rb).buffer_index.wrapping_sub(1usize) as (usize))..];
    *_lhs = _rhs as (u8);
    *_lhs
  };
  i = 0usize;
  while i < kSlackForEightByteHashingEverywhere {
    {
      *(*rb).data_[((*rb)
          .buffer_index
          .wrapping_add((*rb).cur_size_ as (usize))
          .wrapping_add(i) as (usize))..] = 0i32 as (u8);
    }
    i = i.wrapping_add(1 as (usize));
  }
}

fn brotli_min_size_t(mut a: usize, mut b: usize) -> usize {
  if a < b { a } else { b }
}

fn RingBufferWriteTail(mut bytes: &[u8], mut n: usize, mut rb: &mut [RingBuffer]) {
  let masked_pos: usize = ((*rb).pos_ & (*rb).mask_) as (usize);
  if masked_pos < (*rb).tail_size_ as (usize) {
    let p: usize = ((*rb).size_ as (usize)).wrapping_add(masked_pos);
    memcpy(&mut *(*rb).data_[((*rb).buffer_index.wrapping_add(p) as (usize))..],
           bytes,
           brotli_min_size_t(n, ((*rb).tail_size_ as (usize)).wrapping_sub(masked_pos)));
  }
}

fn RingBufferWrite(mut m: &mut [MemoryManager],
                   mut bytes: &[u8],
                   mut n: usize,
                   mut rb: &mut [RingBuffer]) {
  if (*rb).pos_ == 0u32 && (n < (*rb).tail_size_ as (usize)) {
    (*rb).pos_ = n as (u32);
    RingBufferInitBuffer(m, (*rb).pos_, rb);
    if !(0i32 == 0) {
      return;
    }
    memcpy(&mut *(*rb).data_[((*rb).buffer_index as (usize))..],
           bytes,
           n);
    return;
  }
  if (*rb).cur_size_ < (*rb).total_size_ {
    RingBufferInitBuffer(m, (*rb).total_size_, rb);
    if !(0i32 == 0) {
      return;
    }
    *(*rb).data_[((*rb)
        .buffer_index
        .wrapping_add((*rb).size_ as (usize))
        .wrapping_sub(2usize) as (usize))..] = 0i32 as (u8);
    *(*rb).data_[((*rb)
        .buffer_index
        .wrapping_add((*rb).size_ as (usize))
        .wrapping_sub(1usize) as (usize))..] = 0i32 as (u8);
  }
  {
    let masked_pos: usize = ((*rb).pos_ & (*rb).mask_) as (usize);
    RingBufferWriteTail(bytes, n, rb);
    if masked_pos.wrapping_add(n) <= (*rb).size_ as (usize) {
      memcpy(&mut *(*rb).data_[((*rb).buffer_index.wrapping_add(masked_pos) as (usize))..],
             bytes,
             n);
    } else {
      memcpy(&mut *(*rb).data_[((*rb).buffer_index.wrapping_add(masked_pos) as (usize))..],
             bytes,
             brotli_min_size_t(n, ((*rb).total_size_ as (usize)).wrapping_sub(masked_pos)));
      memcpy(&mut *(*rb).data_[((*rb).buffer_index.wrapping_add(0usize) as (usize))..],
             bytes[(((*rb).size_ as (usize)).wrapping_sub(masked_pos) as (usize))..],
             n.wrapping_sub(((*rb).size_ as (usize)).wrapping_sub(masked_pos)));
    }
  }
  *(*rb).data_[((*rb).buffer_index.wrapping_sub(2usize) as (usize))..] =
    *(*rb).data_[((*rb)
        .buffer_index
        .wrapping_add((*rb).size_ as (usize))
        .wrapping_sub(2usize) as (usize))..];
  *(*rb).data_[((*rb).buffer_index.wrapping_sub(1usize) as (usize))..] =
    *(*rb).data_[((*rb)
        .buffer_index
        .wrapping_add((*rb).size_ as (usize))
        .wrapping_sub(1usize) as (usize))..];
  (*rb).pos_ = (*rb).pos_.wrapping_add(n as (u32));
  if (*rb).pos_ > 1u32 << 30i32 {
    (*rb).pos_ = (*rb).pos_ & (1u32 << 30i32).wrapping_sub(1u32) | 1u32 << 30i32;
  }
}

fn CopyInputToRingBuffer(mut s: &mut [BrotliEncoderStateStruct],
                         input_size: usize,
                         mut input_buffer: &[u8]) {
  let mut ringbuffer_: *mut RingBuffer = &mut (*s).ringbuffer_;
  let mut m: *mut MemoryManager = &mut (*s).memory_manager_;
  if EnsureInitialized(s) == 0 {
    return;
  }
  RingBufferWrite(m, input_buffer, input_size, ringbuffer_);
  if !(0i32 == 0) {
    return;
  }
  (*s).input_pos_ = (*s).input_pos_.wrapping_add(input_size);
  if (*ringbuffer_).pos_ <= (*ringbuffer_).mask_ {
    memset(&mut *(*ringbuffer_).data_[((*ringbuffer_).buffer_index.wrapping_add((*ringbuffer_).pos_ as (usize)) as
                  (usize))..],
           0i32,
           7usize);
  }
}



pub struct Struct4 {
  pub params: BrotliHasherParams,
  pub is_prepared_: i32,
  pub dict_num_lookups: usize,
  pub dict_num_matches: usize,
}

fn ChooseHasher(mut params: &[BrotliEncoderParams], mut hparams: &mut [BrotliHasherParams]) {
  if (*params).quality > 9i32 {
    (*hparams).type_ = 10i32;
  } else if (*params).quality == 4i32 && ((*params).size_hint >= (1i32 << 20i32) as (usize)) {
    (*hparams).type_ = 54i32;
  } else if (*params).quality < 5i32 {
    (*hparams).type_ = (*params).quality;
  } else if (*params).lgwin <= 16i32 {
    (*hparams).type_ = if (*params).quality < 7i32 {
      40i32
    } else if (*params).quality < 9i32 {
      41i32
    } else {
      42i32
    };
  } else if (*params).size_hint >= (1i32 << 20i32) as (usize) && ((*params).lgwin >= 19i32) {
    (*hparams).type_ = 6i32;
    (*hparams).block_bits = (*params).quality - 1i32;
    (*hparams).bucket_bits = 15i32;
    (*hparams).hash_len = 5i32;
    (*hparams).num_last_distances_to_check = if (*params).quality < 7i32 {
      4i32
    } else if (*params).quality < 9i32 {
      10i32
    } else {
      16i32
    };
  } else {
    (*hparams).type_ = 5i32;
    (*hparams).block_bits = (*params).quality - 1i32;
    (*hparams).bucket_bits = if (*params).quality < 7i32 {
      14i32
    } else {
      15i32
    };
    (*hparams).num_last_distances_to_check = if (*params).quality < 7i32 {
      4i32
    } else if (*params).quality < 9i32 {
      10i32
    } else {
      16i32
    };
  }
}



pub struct H2 {
  pub buckets_: [u32; 65537],
}

fn HashMemAllocInBytesH2(mut params: &[BrotliEncoderParams],
                         mut one_shot: i32,
                         mut input_size: usize)
                         -> usize {
  params;
  one_shot;
  input_size;
  ::std::mem::size_of::<H2>()
}



pub struct H3 {
  pub buckets_: [u32; 65538],
}

fn HashMemAllocInBytesH3(mut params: &[BrotliEncoderParams],
                         mut one_shot: i32,
                         mut input_size: usize)
                         -> usize {
  params;
  one_shot;
  input_size;
  ::std::mem::size_of::<H3>()
}



pub struct H4 {
  pub buckets_: [u32; 131076],
}

fn HashMemAllocInBytesH4(mut params: &[BrotliEncoderParams],
                         mut one_shot: i32,
                         mut input_size: usize)
                         -> usize {
  params;
  one_shot;
  input_size;
  ::std::mem::size_of::<H4>()
}



pub struct H5 {
  pub bucket_size_: usize,
  pub block_size_: usize,
  pub hash_shift_: i32,
  pub block_mask_: u32,
}

fn HashMemAllocInBytesH5(mut params: &[BrotliEncoderParams],
                         mut one_shot: i32,
                         mut input_size: usize)
                         -> usize {
  let mut bucket_size: usize = 1usize << (*params).hasher.bucket_bits;
  let mut block_size: usize = 1usize << (*params).hasher.block_bits;
  one_shot;
  input_size;
  ::std::mem::size_of::<H5>()
    .wrapping_add(bucket_size.wrapping_mul((2usize).wrapping_add((4usize)
                                                                   .wrapping_mul(block_size))))
}



pub struct H6 {
  pub bucket_size_: usize,
  pub block_size_: usize,
  pub hash_shift_: i32,
  pub hash_mask_: usize,
  pub block_mask_: u32,
}

fn HashMemAllocInBytesH6(mut params: &[BrotliEncoderParams],
                         mut one_shot: i32,
                         mut input_size: usize)
                         -> usize {
  let mut bucket_size: usize = 1usize << (*params).hasher.bucket_bits;
  let mut block_size: usize = 1usize << (*params).hasher.block_bits;
  one_shot;
  input_size;
  ::std::mem::size_of::<H6>()
    .wrapping_add(bucket_size.wrapping_mul((2usize).wrapping_add((4usize)
                                                                   .wrapping_mul(block_size))))
}



pub struct SlotH40 {
  pub delta: u16,
  pub next: u16,
}



pub struct BankH40 {
  pub slots: [SlotH40; 65536],
}



pub struct H40 {
  pub addr: [u32; 32768],
  pub head: [u16; 32768],
  pub tiny_hash: [u8; 65536],
  pub banks: [BankH40; 1],
  pub free_slot_idx: [u16; 1],
  pub max_hops: usize,
}

fn HashMemAllocInBytesH40(mut params: &[BrotliEncoderParams],
                          mut one_shot: i32,
                          mut input_size: usize)
                          -> usize {
  params;
  one_shot;
  input_size;
  ::std::mem::size_of::<H40>()
}



pub struct SlotH41 {
  pub delta: u16,
  pub next: u16,
}



pub struct BankH41 {
  pub slots: [SlotH41; 65536],
}



pub struct H41 {
  pub addr: [u32; 32768],
  pub head: [u16; 32768],
  pub tiny_hash: [u8; 65536],
  pub banks: [BankH41; 1],
  pub free_slot_idx: [u16; 1],
  pub max_hops: usize,
}

fn HashMemAllocInBytesH41(mut params: &[BrotliEncoderParams],
                          mut one_shot: i32,
                          mut input_size: usize)
                          -> usize {
  params;
  one_shot;
  input_size;
  ::std::mem::size_of::<H41>()
}



pub struct SlotH42 {
  pub delta: u16,
  pub next: u16,
}



pub struct BankH42 {
  pub slots: [SlotH42; 512],
}



pub struct H42 {
  pub addr: [u32; 32768],
  pub head: [u16; 32768],
  pub tiny_hash: [u8; 65536],
  pub banks: [BankH42; 512],
  pub free_slot_idx: [u16; 512],
  pub max_hops: usize,
}

fn HashMemAllocInBytesH42(mut params: &[BrotliEncoderParams],
                          mut one_shot: i32,
                          mut input_size: usize)
                          -> usize {
  params;
  one_shot;
  input_size;
  ::std::mem::size_of::<H42>()
}



pub struct H54 {
  pub buckets_: [u32; 1048580],
}

fn HashMemAllocInBytesH54(mut params: &[BrotliEncoderParams],
                          mut one_shot: i32,
                          mut input_size: usize)
                          -> usize {
  params;
  one_shot;
  input_size;
  ::std::mem::size_of::<H54>()
}



pub struct H10 {
  pub window_mask_: usize,
  pub buckets_: [u32; 131072],
  pub invalid_pos_: u32,
}

fn HashMemAllocInBytesH10(mut params: &[BrotliEncoderParams],
                          mut one_shot: i32,
                          mut input_size: usize)
                          -> usize {
  let mut num_nodes: usize = 1usize << (*params).lgwin;
  if one_shot != 0 && (input_size < num_nodes) {
    num_nodes = input_size;
  }
  ::std::mem::size_of::<H10>().wrapping_add((2usize)
                                              .wrapping_mul(::std::mem::size_of::<u32>())
                                              .wrapping_mul(num_nodes))
}

fn HasherSize(mut params: &[BrotliEncoderParams], mut one_shot: i32, input_size: usize) -> usize {
  let mut result: usize = ::std::mem::size_of::<Struct4>();
  let mut hashtype: i32 = (*params).hasher.type_;
  if hashtype == 2i32 {
    result = result.wrapping_add(HashMemAllocInBytesH2(params, one_shot, input_size));
  }
  if hashtype == 3i32 {
    result = result.wrapping_add(HashMemAllocInBytesH3(params, one_shot, input_size));
  }
  if hashtype == 4i32 {
    result = result.wrapping_add(HashMemAllocInBytesH4(params, one_shot, input_size));
  }
  if hashtype == 5i32 {
    result = result.wrapping_add(HashMemAllocInBytesH5(params, one_shot, input_size));
  }
  if hashtype == 6i32 {
    result = result.wrapping_add(HashMemAllocInBytesH6(params, one_shot, input_size));
  }
  if hashtype == 40i32 {
    result = result.wrapping_add(HashMemAllocInBytesH40(params, one_shot, input_size));
  }
  if hashtype == 41i32 {
    result = result.wrapping_add(HashMemAllocInBytesH41(params, one_shot, input_size));
  }
  if hashtype == 42i32 {
    result = result.wrapping_add(HashMemAllocInBytesH42(params, one_shot, input_size));
  }
  if hashtype == 54i32 {
    result = result.wrapping_add(HashMemAllocInBytesH54(params, one_shot, input_size));
  }
  if hashtype == 10i32 {
    result = result.wrapping_add(HashMemAllocInBytesH10(params, one_shot, input_size));
  }
  result
}

fn GetHasherCommon(mut handle: &mut [u8]) -> *mut Struct4 {
  handle
}

fn InitializeH2(mut handle: &mut [u8], mut params: &[BrotliEncoderParams]) {
  handle;
  params;
}

fn InitializeH3(mut handle: &mut [u8], mut params: &[BrotliEncoderParams]) {
  handle;
  params;
}

fn InitializeH4(mut handle: &mut [u8], mut params: &[BrotliEncoderParams]) {
  handle;
  params;
}

fn SelfH5(mut handle: &mut [u8]) -> *mut H5 {
  &mut *GetHasherCommon(handle).offset(1i32 as (isize))
}

fn InitializeH5(mut handle: &mut [u8], mut params: &[BrotliEncoderParams]) {
  let mut common: *mut Struct4 = GetHasherCommon(handle);
  let mut xself: *mut H5 = SelfH5(handle);
  params;
  (*xself).hash_shift_ = 32i32 - (*common).params.bucket_bits;
  (*xself).bucket_size_ = 1usize << (*common).params.bucket_bits;
  (*xself).block_size_ = 1usize << (*common).params.block_bits;
  (*xself).block_mask_ = (*xself).block_size_.wrapping_sub(1usize) as (u32);
}

fn SelfH6(mut handle: &mut [u8]) -> *mut H6 {
  &mut *GetHasherCommon(handle).offset(1i32 as (isize))
}

fn InitializeH6(mut handle: &mut [u8], mut params: &[BrotliEncoderParams]) {
  let mut common: *mut Struct4 = GetHasherCommon(handle);
  let mut xself: *mut H6 = SelfH6(handle);
  params;
  (*xself).hash_shift_ = 64i32 - (*common).params.bucket_bits;
  (*xself).hash_mask_ = !(0u32 as (usize)) >> 64i32 - 8i32 * (*common).params.hash_len;
  (*xself).bucket_size_ = 1usize << (*common).params.bucket_bits;
  (*xself).block_size_ = 1usize << (*common).params.block_bits;
  (*xself).block_mask_ = (*xself).block_size_.wrapping_sub(1usize) as (u32);
}

fn SelfH40(mut handle: &mut [u8]) -> *mut H40 {
  &mut *GetHasherCommon(handle).offset(1i32 as (isize))
}

fn InitializeH40(mut handle: &mut [u8], mut params: &[BrotliEncoderParams]) {
  (*SelfH40(handle)).max_hops =
    (if (*params).quality > 6i32 { 7u32 } else { 8u32 } << (*params).quality - 4i32) as (usize);
}

fn SelfH41(mut handle: &mut [u8]) -> *mut H41 {
  &mut *GetHasherCommon(handle).offset(1i32 as (isize))
}

fn InitializeH41(mut handle: &mut [u8], mut params: &[BrotliEncoderParams]) {
  (*SelfH41(handle)).max_hops =
    (if (*params).quality > 6i32 { 7u32 } else { 8u32 } << (*params).quality - 4i32) as (usize);
}

fn SelfH42(mut handle: &mut [u8]) -> *mut H42 {
  &mut *GetHasherCommon(handle).offset(1i32 as (isize))
}

fn InitializeH42(mut handle: &mut [u8], mut params: &[BrotliEncoderParams]) {
  (*SelfH42(handle)).max_hops =
    (if (*params).quality > 6i32 { 7u32 } else { 8u32 } << (*params).quality - 4i32) as (usize);
}

fn InitializeH54(mut handle: &mut [u8], mut params: &[BrotliEncoderParams]) {
  handle;
  params;
}

fn SelfH10(mut handle: &mut [u8]) -> *mut H10 {
  &mut *GetHasherCommon(handle).offset(1i32 as (isize))
}

fn InitializeH10(mut handle: &mut [u8], mut params: &[BrotliEncoderParams]) {
  let mut xself: *mut H10 = SelfH10(handle);
  (*xself).window_mask_ = (1u32 << (*params).lgwin).wrapping_sub(1u32) as (usize);
  (*xself).invalid_pos_ = (0usize).wrapping_sub((*xself).window_mask_) as (u32);
}

fn HasherReset(mut handle: &mut [u8]) {
  if handle == 0i32 {
    return;
  }
  (*GetHasherCommon(handle)).is_prepared_ = 0i32;
}

fn SelfH2(mut handle: &mut [u8]) -> *mut H2 {
  &mut *GetHasherCommon(handle).offset(1i32 as (isize))
}

fn BROTLI_UNALIGNED_LOAD64(mut p: &[::std::os::raw::c_void]) -> usize {
  let mut t: usize;
  memcpy(&mut t, p, ::std::mem::size_of::<usize>());
  t
}

fn HashBytesH2(mut data: &[u8]) -> u32 {
  let h: usize = (BROTLI_UNALIGNED_LOAD64(data) << 64i32 - 8i32 * 5i32).wrapping_mul(kHashMul64);
  (h >> 64i32 - 16i32) as (u32)
}

fn PrepareH2(mut handle: &mut [u8], mut one_shot: i32, mut input_size: usize, mut data: &[u8]) {
  let mut xself: *mut H2 = SelfH2(handle);
  let mut partial_prepare_threshold: usize = (4i32 << 16i32 >> 7i32) as (usize);
  if one_shot != 0 && (input_size <= partial_prepare_threshold) {
    let mut i: usize;
    i = 0usize;
    while i < input_size {
      {
        let key: u32 = HashBytesH2(&data[(i as (usize))]);
        memset(&mut (*xself).buckets_[key as (usize)],
               0i32,
               (1usize).wrapping_mul(::std::mem::size_of::<u32>()));
      }
      i = i.wrapping_add(1 as (usize));
    }
  } else {
    memset(&mut (*xself).buckets_[0usize],
           0i32,
           ::std::mem::size_of::<[u32; 65537]>());
  }
}

fn SelfH3(mut handle: &mut [u8]) -> *mut H3 {
  &mut *GetHasherCommon(handle).offset(1i32 as (isize))
}

fn HashBytesH3(mut data: &[u8]) -> u32 {
  let h: usize = (BROTLI_UNALIGNED_LOAD64(data) << 64i32 - 8i32 * 5i32).wrapping_mul(kHashMul64);
  (h >> 64i32 - 16i32) as (u32)
}

fn PrepareH3(mut handle: &mut [u8], mut one_shot: i32, mut input_size: usize, mut data: &[u8]) {
  let mut xself: *mut H3 = SelfH3(handle);
  let mut partial_prepare_threshold: usize = (4i32 << 16i32 >> 7i32) as (usize);
  if one_shot != 0 && (input_size <= partial_prepare_threshold) {
    let mut i: usize;
    i = 0usize;
    while i < input_size {
      {
        let key: u32 = HashBytesH3(&data[(i as (usize))]);
        memset(&mut (*xself).buckets_[key as (usize)],
               0i32,
               (2usize).wrapping_mul(::std::mem::size_of::<u32>()));
      }
      i = i.wrapping_add(1 as (usize));
    }
  } else {
    memset(&mut (*xself).buckets_[0usize],
           0i32,
           ::std::mem::size_of::<[u32; 65538]>());
  }
}

fn SelfH4(mut handle: &mut [u8]) -> *mut H4 {
  &mut *GetHasherCommon(handle).offset(1i32 as (isize))
}

fn HashBytesH4(mut data: &[u8]) -> u32 {
  let h: usize = (BROTLI_UNALIGNED_LOAD64(data) << 64i32 - 8i32 * 5i32).wrapping_mul(kHashMul64);
  (h >> 64i32 - 17i32) as (u32)
}

fn PrepareH4(mut handle: &mut [u8], mut one_shot: i32, mut input_size: usize, mut data: &[u8]) {
  let mut xself: *mut H4 = SelfH4(handle);
  let mut partial_prepare_threshold: usize = (4i32 << 17i32 >> 7i32) as (usize);
  if one_shot != 0 && (input_size <= partial_prepare_threshold) {
    let mut i: usize;
    i = 0usize;
    while i < input_size {
      {
        let key: u32 = HashBytesH4(&data[(i as (usize))]);
        memset(&mut (*xself).buckets_[key as (usize)],
               0i32,
               (4usize).wrapping_mul(::std::mem::size_of::<u32>()));
      }
      i = i.wrapping_add(1 as (usize));
    }
  } else {
    memset(&mut (*xself).buckets_[0usize],
           0i32,
           ::std::mem::size_of::<[u32; 131076]>());
  }
}

fn NumH5(mut xself: &mut H5) -> *mut u16 {
  &mut xself[(1usize)]
}

fn BROTLI_UNALIGNED_LOAD32(mut p: &[::std::os::raw::c_void]) -> u32 {
  let mut t: u32;
  memcpy(&mut t, p, ::std::mem::size_of::<u32>());
  t
}

fn HashBytesH5(mut data: &[u8], shift: i32) -> u32 {
  let mut h: u32 = BROTLI_UNALIGNED_LOAD32(data).wrapping_mul(kHashMul32);
  h >> shift
}

fn PrepareH5(mut handle: &mut [u8], mut one_shot: i32, mut input_size: usize, mut data: &[u8]) {
  let mut xself: *mut H5 = SelfH5(handle);
  let mut num: *mut u16 = NumH5(xself);
  let mut partial_prepare_threshold: usize = (*xself).bucket_size_ >> 6i32;
  if one_shot != 0 && (input_size <= partial_prepare_threshold) {
    let mut i: usize;
    i = 0usize;
    while i < input_size {
      {
        let key: u32 = HashBytesH5(&data[(i as (usize))], (*xself).hash_shift_);
        num[(key as (usize))] = 0i32 as (u16);
      }
      i = i.wrapping_add(1 as (usize));
    }
  } else {
    memset(num,
           0i32,
           (*xself).bucket_size_.wrapping_mul(::std::mem::size_of::<u16>()));
  }
}

fn NumH6(mut xself: &mut H6) -> *mut u16 {
  &mut xself[(1usize)]
}

fn HashBytesH6(mut data: &[u8], mask: usize, shift: i32) -> u32 {
  let h: usize = (BROTLI_UNALIGNED_LOAD64(data) & mask).wrapping_mul(kHashMul64Long);
  (h >> shift) as (u32)
}

fn PrepareH6(mut handle: &mut [u8], mut one_shot: i32, mut input_size: usize, mut data: &[u8]) {
  let mut xself: *mut H6 = SelfH6(handle);
  let mut num: *mut u16 = NumH6(xself);
  let mut partial_prepare_threshold: usize = (*xself).bucket_size_ >> 6i32;
  if one_shot != 0 && (input_size <= partial_prepare_threshold) {
    let mut i: usize;
    i = 0usize;
    while i < input_size {
      {
        let key: u32 = HashBytesH6(&data[(i as (usize))],
                                   (*xself).hash_mask_,
                                   (*xself).hash_shift_);
        num[(key as (usize))] = 0i32 as (u16);
      }
      i = i.wrapping_add(1 as (usize));
    }
  } else {
    memset(num,
           0i32,
           (*xself).bucket_size_.wrapping_mul(::std::mem::size_of::<u16>()));
  }
}

fn HashBytesH40(mut data: &[u8]) -> usize {
  let h: u32 = BROTLI_UNALIGNED_LOAD32(data).wrapping_mul(kHashMul32);
  (h >> 32i32 - 15i32) as (usize)
}

fn PrepareH40(mut handle: &mut [u8], mut one_shot: i32, mut input_size: usize, mut data: &[u8]) {
  let mut xself: *mut H40 = SelfH40(handle);
  let mut partial_prepare_threshold: usize = (32768i32 >> 6i32) as (usize);
  if one_shot != 0 && (input_size <= partial_prepare_threshold) {
    let mut i: usize;
    i = 0usize;
    while i < input_size {
      {
        let mut bucket: usize = HashBytesH40(&data[(i as (usize))]);
        (*xself).addr[bucket] = 0xccccccccu32;
        (*xself).head[bucket] = 0xcccci32 as (u16);
      }
      i = i.wrapping_add(1 as (usize));
    }
  } else {
    memset((*xself).addr.as_mut_ptr(),
           0xcci32,
           ::std::mem::size_of::<[u32; 32768]>());
    memset((*xself).head.as_mut_ptr(),
           0i32,
           ::std::mem::size_of::<[u16; 32768]>());
  }
  memset((*xself).tiny_hash.as_mut_ptr(),
         0i32,
         ::std::mem::size_of::<[u8; 65536]>());
  memset((*xself).free_slot_idx.as_mut_ptr(),
         0i32,
         ::std::mem::size_of::<[u16; 1]>());
}

fn HashBytesH41(mut data: &[u8]) -> usize {
  let h: u32 = BROTLI_UNALIGNED_LOAD32(data).wrapping_mul(kHashMul32);
  (h >> 32i32 - 15i32) as (usize)
}

fn PrepareH41(mut handle: &mut [u8], mut one_shot: i32, mut input_size: usize, mut data: &[u8]) {
  let mut xself: *mut H41 = SelfH41(handle);
  let mut partial_prepare_threshold: usize = (32768i32 >> 6i32) as (usize);
  if one_shot != 0 && (input_size <= partial_prepare_threshold) {
    let mut i: usize;
    i = 0usize;
    while i < input_size {
      {
        let mut bucket: usize = HashBytesH41(&data[(i as (usize))]);
        (*xself).addr[bucket] = 0xccccccccu32;
        (*xself).head[bucket] = 0xcccci32 as (u16);
      }
      i = i.wrapping_add(1 as (usize));
    }
  } else {
    memset((*xself).addr.as_mut_ptr(),
           0xcci32,
           ::std::mem::size_of::<[u32; 32768]>());
    memset((*xself).head.as_mut_ptr(),
           0i32,
           ::std::mem::size_of::<[u16; 32768]>());
  }
  memset((*xself).tiny_hash.as_mut_ptr(),
         0i32,
         ::std::mem::size_of::<[u8; 65536]>());
  memset((*xself).free_slot_idx.as_mut_ptr(),
         0i32,
         ::std::mem::size_of::<[u16; 1]>());
}

fn HashBytesH42(mut data: &[u8]) -> usize {
  let h: u32 = BROTLI_UNALIGNED_LOAD32(data).wrapping_mul(kHashMul32);
  (h >> 32i32 - 15i32) as (usize)
}

fn PrepareH42(mut handle: &mut [u8], mut one_shot: i32, mut input_size: usize, mut data: &[u8]) {
  let mut xself: *mut H42 = SelfH42(handle);
  let mut partial_prepare_threshold: usize = (32768i32 >> 6i32) as (usize);
  if one_shot != 0 && (input_size <= partial_prepare_threshold) {
    let mut i: usize;
    i = 0usize;
    while i < input_size {
      {
        let mut bucket: usize = HashBytesH42(&data[(i as (usize))]);
        (*xself).addr[bucket] = 0xccccccccu32;
        (*xself).head[bucket] = 0xcccci32 as (u16);
      }
      i = i.wrapping_add(1 as (usize));
    }
  } else {
    memset((*xself).addr.as_mut_ptr(),
           0xcci32,
           ::std::mem::size_of::<[u32; 32768]>());
    memset((*xself).head.as_mut_ptr(),
           0i32,
           ::std::mem::size_of::<[u16; 32768]>());
  }
  memset((*xself).tiny_hash.as_mut_ptr(),
         0i32,
         ::std::mem::size_of::<[u8; 65536]>());
  memset((*xself).free_slot_idx.as_mut_ptr(),
         0i32,
         ::std::mem::size_of::<[u16; 512]>());
}

fn SelfH54(mut handle: &mut [u8]) -> *mut H54 {
  &mut *GetHasherCommon(handle).offset(1i32 as (isize))
}

fn HashBytesH54(mut data: &[u8]) -> u32 {
  let h: usize = (BROTLI_UNALIGNED_LOAD64(data) << 64i32 - 8i32 * 7i32).wrapping_mul(kHashMul64);
  (h >> 64i32 - 20i32) as (u32)
}

fn PrepareH54(mut handle: &mut [u8], mut one_shot: i32, mut input_size: usize, mut data: &[u8]) {
  let mut xself: *mut H54 = SelfH54(handle);
  let mut partial_prepare_threshold: usize = (4i32 << 20i32 >> 7i32) as (usize);
  if one_shot != 0 && (input_size <= partial_prepare_threshold) {
    let mut i: usize;
    i = 0usize;
    while i < input_size {
      {
        let key: u32 = HashBytesH54(&data[(i as (usize))]);
        memset(&mut (*xself).buckets_[key as (usize)],
               0i32,
               (4usize).wrapping_mul(::std::mem::size_of::<u32>()));
      }
      i = i.wrapping_add(1 as (usize));
    }
  } else {
    memset(&mut (*xself).buckets_[0usize],
           0i32,
           ::std::mem::size_of::<[u32; 1048580]>());
  }
}

fn PrepareH10(mut handle: &mut [u8], mut one_shot: i32, mut input_size: usize, mut data: &[u8]) {
  let mut xself: *mut H10 = SelfH10(handle);
  let mut invalid_pos: u32 = (*xself).invalid_pos_;
  let mut i: u32;
  data;
  one_shot;
  input_size;
  i = 0u32;
  while i < (1i32 << 17i32) as (u32) {
    {
      (*xself).buckets_[i as (usize)] = invalid_pos;
    }
    i = i.wrapping_add(1 as (u32));
  }
}

fn HasherSetup(mut m: &mut [MemoryManager],
               mut handle: &mut [*mut u8],
               mut params: &mut [BrotliEncoderParams],
               mut data: &[u8],
               mut position: usize,
               mut input_size: usize,
               mut is_last: i32) {
  let mut xself: *mut u8 = 0i32;
  let mut common: *mut Struct4 = 0i32;
  let mut one_shot: i32 = (position == 0usize && (is_last != 0)) as (i32);
  if *handle == 0i32 {
    let mut alloc_size: usize;
    ChooseHasher(params, &mut (*params).hasher);
    alloc_size = HasherSize(params, one_shot, input_size);
    xself = if alloc_size != 0 {
      BrotliAllocate(m, alloc_size.wrapping_mul(::std::mem::size_of::<u8>()))
    } else {
      0i32
    };
    if !(0i32 == 0) {
      return;
    }
    *handle = xself;
    common = GetHasherCommon(xself);
    (*common).params = (*params).hasher;
    let mut hasher_type: i32 = (*common).params.type_;
    if hasher_type == 2i32 {
      InitializeH2(*handle, params);
    }
    if hasher_type == 3i32 {
      InitializeH3(*handle, params);
    }
    if hasher_type == 4i32 {
      InitializeH4(*handle, params);
    }
    if hasher_type == 5i32 {
      InitializeH5(*handle, params);
    }
    if hasher_type == 6i32 {
      InitializeH6(*handle, params);
    }
    if hasher_type == 40i32 {
      InitializeH40(*handle, params);
    }
    if hasher_type == 41i32 {
      InitializeH41(*handle, params);
    }
    if hasher_type == 42i32 {
      InitializeH42(*handle, params);
    }
    if hasher_type == 54i32 {
      InitializeH54(*handle, params);
    }
    if hasher_type == 10i32 {
      InitializeH10(*handle, params);
    }
    HasherReset(*handle);
  }
  xself = *handle;
  common = GetHasherCommon(xself);
  if (*common).is_prepared_ == 0 {
    let mut hasher_type: i32 = (*common).params.type_;
    if hasher_type == 2i32 {
      PrepareH2(xself, one_shot, input_size, data);
    }
    if hasher_type == 3i32 {
      PrepareH3(xself, one_shot, input_size, data);
    }
    if hasher_type == 4i32 {
      PrepareH4(xself, one_shot, input_size, data);
    }
    if hasher_type == 5i32 {
      PrepareH5(xself, one_shot, input_size, data);
    }
    if hasher_type == 6i32 {
      PrepareH6(xself, one_shot, input_size, data);
    }
    if hasher_type == 40i32 {
      PrepareH40(xself, one_shot, input_size, data);
    }
    if hasher_type == 41i32 {
      PrepareH41(xself, one_shot, input_size, data);
    }
    if hasher_type == 42i32 {
      PrepareH42(xself, one_shot, input_size, data);
    }
    if hasher_type == 54i32 {
      PrepareH54(xself, one_shot, input_size, data);
    }
    if hasher_type == 10i32 {
      PrepareH10(xself, one_shot, input_size, data);
    }
    if position == 0usize {
      (*common).dict_num_lookups = 0usize;
      (*common).dict_num_matches = 0usize;
    }
    (*common).is_prepared_ = 1i32;
  }
}

fn StoreLookaheadH2() -> usize {
  8usize
}

fn StoreH2(mut handle: &mut [u8], mut data: &[u8], mask: usize, ix: usize) {
  let key: u32 = HashBytesH2(&data[((ix & mask) as (usize))]);
  let off: u32 = (ix >> 3i32).wrapping_rem(1usize) as (u32);
  (*SelfH2(handle)).buckets_[key.wrapping_add(off) as (usize)] = ix as (u32);
}

fn StoreLookaheadH3() -> usize {
  8usize
}

fn StoreH3(mut handle: &mut [u8], mut data: &[u8], mask: usize, ix: usize) {
  let key: u32 = HashBytesH3(&data[((ix & mask) as (usize))]);
  let off: u32 = (ix >> 3i32).wrapping_rem(2usize) as (u32);
  (*SelfH3(handle)).buckets_[key.wrapping_add(off) as (usize)] = ix as (u32);
}

fn StoreLookaheadH4() -> usize {
  8usize
}

fn StoreH4(mut handle: &mut [u8], mut data: &[u8], mask: usize, ix: usize) {
  let key: u32 = HashBytesH4(&data[((ix & mask) as (usize))]);
  let off: u32 = (ix >> 3i32).wrapping_rem(4usize) as (u32);
  (*SelfH4(handle)).buckets_[key.wrapping_add(off) as (usize)] = ix as (u32);
}

fn StoreLookaheadH5() -> usize {
  4usize
}

fn BucketsH5(mut xself: &mut H5) -> *mut u32 {
  &mut *NumH5(xself).offset((*xself).bucket_size_ as (isize))
}

fn StoreH5(mut handle: &mut [u8], mut data: &[u8], mask: usize, ix: usize) {
  let mut xself: *mut H5 = SelfH5(handle);
  let mut num: *mut u16 = NumH5(xself);
  let key: u32 = HashBytesH5(&data[((ix & mask) as (usize))], (*xself).hash_shift_);
  let minor_ix: usize = (num[(key as (usize))] as (u32) & (*xself).block_mask_) as (usize);
  let offset: usize = minor_ix.wrapping_add((key << (*GetHasherCommon(handle)).params.block_bits) as
                                            (usize));
  *BucketsH5(xself).offset(offset as (isize)) = ix as (u32);
  {
    let _rhs = 1;
    let _lhs = &mut num[(key as (usize))];
    *_lhs = (*_lhs as (i32) + _rhs) as (u16);
  }
}

fn StoreLookaheadH6() -> usize {
  8usize
}

fn BucketsH6(mut xself: &mut H6) -> *mut u32 {
  &mut *NumH6(xself).offset((*xself).bucket_size_ as (isize))
}

fn StoreH6(mut handle: &mut [u8], mut data: &[u8], mask: usize, ix: usize) {
  let mut xself: *mut H6 = SelfH6(handle);
  let mut num: *mut u16 = NumH6(xself);
  let key: u32 = HashBytesH6(&data[((ix & mask) as (usize))],
                             (*xself).hash_mask_,
                             (*xself).hash_shift_);
  let minor_ix: usize = (num[(key as (usize))] as (u32) & (*xself).block_mask_) as (usize);
  let offset: usize = minor_ix.wrapping_add((key << (*GetHasherCommon(handle)).params.block_bits) as
                                            (usize));
  *BucketsH6(xself).offset(offset as (isize)) = ix as (u32);
  {
    let _rhs = 1;
    let _lhs = &mut num[(key as (usize))];
    *_lhs = (*_lhs as (i32) + _rhs) as (u16);
  }
}

fn StoreLookaheadH40() -> usize {
  4usize
}

fn StoreH40(mut handle: &mut [u8], mut data: &[u8], mask: usize, ix: usize) {
  let mut xself: *mut H40 = SelfH40(handle);
  let key: usize = HashBytesH40(&data[((ix & mask) as (usize))]);
  let bank: usize = key & (1i32 - 1i32) as (usize);
  let idx: usize = (({
                       let _rhs = 1;
                       let _lhs = &mut (*xself).free_slot_idx[bank];
                       let _old = *_lhs;
                       *_lhs = (*_lhs as (i32) + _rhs) as (u16);
                       _old
                     }) as (i32) & 65536i32 - 1i32) as (usize);
  let mut delta: usize = ix.wrapping_sub((*xself).addr[key] as (usize));
  (*xself).tiny_hash[ix as (u16) as (usize)] = key as (u8);
  if delta > 0xffffusize {
    delta = if 0i32 != 0 { 0i32 } else { 0xffffi32 } as (usize);
  }
  (*xself).banks[bank].slots[idx].delta = delta as (u16);
  (*xself).banks[bank].slots[idx].next = (*xself).head[key];
  (*xself).addr[key] = ix as (u32);
  (*xself).head[key] = idx as (u16);
}

fn StoreLookaheadH41() -> usize {
  4usize
}

fn StoreH41(mut handle: &mut [u8], mut data: &[u8], mask: usize, ix: usize) {
  let mut xself: *mut H41 = SelfH41(handle);
  let key: usize = HashBytesH41(&data[((ix & mask) as (usize))]);
  let bank: usize = key & (1i32 - 1i32) as (usize);
  let idx: usize = (({
                       let _rhs = 1;
                       let _lhs = &mut (*xself).free_slot_idx[bank];
                       let _old = *_lhs;
                       *_lhs = (*_lhs as (i32) + _rhs) as (u16);
                       _old
                     }) as (i32) & 65536i32 - 1i32) as (usize);
  let mut delta: usize = ix.wrapping_sub((*xself).addr[key] as (usize));
  (*xself).tiny_hash[ix as (u16) as (usize)] = key as (u8);
  if delta > 0xffffusize {
    delta = if 0i32 != 0 { 0i32 } else { 0xffffi32 } as (usize);
  }
  (*xself).banks[bank].slots[idx].delta = delta as (u16);
  (*xself).banks[bank].slots[idx].next = (*xself).head[key];
  (*xself).addr[key] = ix as (u32);
  (*xself).head[key] = idx as (u16);
}

fn StoreLookaheadH42() -> usize {
  4usize
}

fn StoreH42(mut handle: &mut [u8], mut data: &[u8], mask: usize, ix: usize) {
  let mut xself: *mut H42 = SelfH42(handle);
  let key: usize = HashBytesH42(&data[((ix & mask) as (usize))]);
  let bank: usize = key & (512i32 - 1i32) as (usize);
  let idx: usize = (({
                       let _rhs = 1;
                       let _lhs = &mut (*xself).free_slot_idx[bank];
                       let _old = *_lhs;
                       *_lhs = (*_lhs as (i32) + _rhs) as (u16);
                       _old
                     }) as (i32) & 512i32 - 1i32) as (usize);
  let mut delta: usize = ix.wrapping_sub((*xself).addr[key] as (usize));
  (*xself).tiny_hash[ix as (u16) as (usize)] = key as (u8);
  if delta > 0xffffusize {
    delta = if 0i32 != 0 { 0i32 } else { 0xffffi32 } as (usize);
  }
  (*xself).banks[bank].slots[idx].delta = delta as (u16);
  (*xself).banks[bank].slots[idx].next = (*xself).head[key];
  (*xself).addr[key] = ix as (u32);
  (*xself).head[key] = idx as (u16);
}

fn StoreLookaheadH54() -> usize {
  8usize
}

fn StoreH54(mut handle: &mut [u8], mut data: &[u8], mask: usize, ix: usize) {
  let key: u32 = HashBytesH54(&data[((ix & mask) as (usize))]);
  let off: u32 = (ix >> 3i32).wrapping_rem(4usize) as (u32);
  (*SelfH54(handle)).buckets_[key.wrapping_add(off) as (usize)] = ix as (u32);
}

fn StoreLookaheadH10() -> usize {
  128usize
}



pub struct BackwardMatch {
  pub distance: u32,
  pub length_and_code: u32,
}

fn HashBytesH10(mut data: &[u8]) -> u32 {
  let mut h: u32 = BROTLI_UNALIGNED_LOAD32(data).wrapping_mul(kHashMul32);
  h >> 32i32 - 17i32
}

fn ForestH10(mut xself: &mut H10) -> *mut u32 {
  &mut xself[(1usize)]
}

fn LeftChildIndexH10(mut xself: &mut H10, pos: usize) -> usize {
  (2usize).wrapping_mul(pos & (*xself).window_mask_)
}

fn RightChildIndexH10(mut xself: &mut H10, pos: usize) -> usize {
  (2usize).wrapping_mul(pos & (*xself).window_mask_).wrapping_add(1usize)
}

fn unopt_ctzll(mut val: usize) -> u8 {
  let mut cnt: u8 = 0i32 as (u8);
  while val & 1usize == 0usize {
    val = val >> 1i32;
    cnt = (cnt as (i32) + 1) as (u8);
  }
  cnt
}

fn FindMatchLengthWithLimit(mut s1: &[u8], mut s2: &[u8], mut limit: usize) -> usize {
  let mut matched: usize = 0usize;
  let mut limit2: usize = (limit >> 3i32).wrapping_add(1usize);
  while {
          limit2 = limit2.wrapping_sub(1 as (usize));
          limit2
        } != 0 {
    if BROTLI_UNALIGNED_LOAD64(s2) == BROTLI_UNALIGNED_LOAD64(s1[(matched as (usize))..]) {
      s2 = s2[(8usize)..];
      matched = matched.wrapping_add(8usize);
    } else {
      let mut x: usize = BROTLI_UNALIGNED_LOAD64(s2) ^
                         BROTLI_UNALIGNED_LOAD64(s1[(matched as (usize))..]);
      let mut matching_bits: usize = unopt_ctzll(x) as (usize);
      matched = matched.wrapping_add(matching_bits >> 3i32);
      return matched;
    }
  }
  limit = (limit & 7usize).wrapping_add(1usize);
  while {
          limit = limit.wrapping_sub(1 as (usize));
          limit
        } != 0 {
    if s1[(matched as (usize))] as (i32) == *s2 as (i32) {
      s2 = s2[(1 as (usize))..];
      matched = matched.wrapping_add(1 as (usize));
    } else {
      return matched;
    }
  }
  matched
}

fn InitBackwardMatch(mut xself: &mut BackwardMatch, mut dist: usize, mut len: usize) {
  (*xself).distance = dist as (u32);
  (*xself).length_and_code = (len << 5i32) as (u32);
}

fn StoreAndFindMatchesH10(mut xself: &mut H10,
                          data: &[u8],
                          cur_ix: usize,
                          ring_buffer_mask: usize,
                          max_length: usize,
                          max_backward: usize,
                          best_len: &mut [usize],
                          mut matches: &mut [BackwardMatch])
                          -> *mut BackwardMatch {
  let cur_ix_masked: usize = cur_ix & ring_buffer_mask;
  let max_comp_len: usize = brotli_min_size_t(max_length, 128usize);
  let should_reroot_tree: i32 = if !!(max_length >= 128usize) {
    1i32
  } else {
    0i32
  };
  let key: u32 = HashBytesH10(&data[(cur_ix_masked as (usize))]);
  let mut forest: *mut u32 = ForestH10(xself);
  let mut prev_ix: usize = (*xself).buckets_[key as (usize)] as (usize);
  let mut node_left: usize = LeftChildIndexH10(xself, cur_ix);
  let mut node_right: usize = RightChildIndexH10(xself, cur_ix);
  let mut best_len_left: usize = 0usize;
  let mut best_len_right: usize = 0usize;
  let mut depth_remaining: usize;
  if should_reroot_tree != 0 {
    (*xself).buckets_[key as (usize)] = cur_ix as (u32);
  }
  depth_remaining = 64usize;
  'break45: loop {
    {
      let backward: usize = cur_ix.wrapping_sub(prev_ix);
      let prev_ix_masked: usize = prev_ix & ring_buffer_mask;
      if backward == 0usize || backward > max_backward || depth_remaining == 0usize {
        if should_reroot_tree != 0 {
          forest[(node_left as (usize))] = (*xself).invalid_pos_;
          forest[(node_right as (usize))] = (*xself).invalid_pos_;
        }
        {
          {
            break 'break45;
          }
        }
      }
      {
        let cur_len: usize = brotli_min_size_t(best_len_left, best_len_right);
        let mut len: usize;
        0i32;
        len = cur_len.wrapping_add(
                          FindMatchLengthWithLimit(
                              &data[(
                                    cur_ix_masked.wrapping_add(cur_len) as (usize)
                                ) ],
                              &data[(
                                    prev_ix_masked.wrapping_add(cur_len) as (usize)
                                ) ],
                              max_length.wrapping_sub(cur_len)
                          )
                      );
        0i32;
        if !matches.is_null() && (len > *best_len) {
          *best_len = len;
          InitBackwardMatch({
                              let _old = matches;
                              matches = matches[(1 as (usize))..];
                              _old
                            },
                            backward,
                            len);
        }
        if len >= max_comp_len {
          if should_reroot_tree != 0 {
            forest[(node_left as (usize))] = forest[(LeftChildIndexH10(xself, prev_ix) as (usize))];
            forest[(node_right as (usize))] = forest[(RightChildIndexH10(xself, prev_ix) as
             (usize))];
          }
          {
            {
              break 'break45;
            }
          }
        }
        if data[(cur_ix_masked.wrapping_add(len) as (usize))] as (i32) >
           data[(prev_ix_masked.wrapping_add(len) as (usize))] as (i32) {
          best_len_left = len;
          if should_reroot_tree != 0 {
            forest[(node_left as (usize))] = prev_ix as (u32);
          }
          node_left = RightChildIndexH10(xself, prev_ix);
          prev_ix = forest[(node_left as (usize))] as (usize);
        } else {
          best_len_right = len;
          if should_reroot_tree != 0 {
            forest[(node_right as (usize))] = prev_ix as (u32);
          }
          node_right = LeftChildIndexH10(xself, prev_ix);
          prev_ix = forest[(node_right as (usize))] as (usize);
        }
      }
    }
    depth_remaining = depth_remaining.wrapping_sub(1 as (usize));
  }
  matches
}

fn StoreH10(mut handle: &mut [u8], mut data: &[u8], mask: usize, ix: usize) {
  let mut xself: *mut H10 = SelfH10(handle);
  let max_backward: usize = (*xself).window_mask_.wrapping_sub(16usize).wrapping_add(1usize);
  StoreAndFindMatchesH10(xself, data, ix, mask, 128usize, max_backward, 0i32, 0i32);
}

fn HasherPrependCustomDictionary(mut m: &mut [MemoryManager],
                                 mut handle: &mut [*mut u8],
                                 mut params: &mut [BrotliEncoderParams],
                                 size: usize,
                                 mut dict: &[u8]) {
  let mut overlap: usize;
  let mut i: usize;
  let mut xself: *mut u8;
  HasherSetup(m, handle, params, dict, 0usize, size, 0i32);
  if !(0i32 == 0) {
    return;
  }
  xself = *handle;
  let mut hasher_type: i32 = (*GetHasherCommon(xself)).params.type_;
  if hasher_type == 2i32 {
    overlap = StoreLookaheadH2().wrapping_sub(1usize);
    i = 0usize;
    while i.wrapping_add(overlap) < size {
      {
        StoreH2(xself, dict, !(0usize), i);
      }
      i = i.wrapping_add(1 as (usize));
    }
  }
  if hasher_type == 3i32 {
    overlap = StoreLookaheadH3().wrapping_sub(1usize);
    i = 0usize;
    while i.wrapping_add(overlap) < size {
      {
        StoreH3(xself, dict, !(0usize), i);
      }
      i = i.wrapping_add(1 as (usize));
    }
  }
  if hasher_type == 4i32 {
    overlap = StoreLookaheadH4().wrapping_sub(1usize);
    i = 0usize;
    while i.wrapping_add(overlap) < size {
      {
        StoreH4(xself, dict, !(0usize), i);
      }
      i = i.wrapping_add(1 as (usize));
    }
  }
  if hasher_type == 5i32 {
    overlap = StoreLookaheadH5().wrapping_sub(1usize);
    i = 0usize;
    while i.wrapping_add(overlap) < size {
      {
        StoreH5(xself, dict, !(0usize), i);
      }
      i = i.wrapping_add(1 as (usize));
    }
  }
  if hasher_type == 6i32 {
    overlap = StoreLookaheadH6().wrapping_sub(1usize);
    i = 0usize;
    while i.wrapping_add(overlap) < size {
      {
        StoreH6(xself, dict, !(0usize), i);
      }
      i = i.wrapping_add(1 as (usize));
    }
  }
  if hasher_type == 40i32 {
    overlap = StoreLookaheadH40().wrapping_sub(1usize);
    i = 0usize;
    while i.wrapping_add(overlap) < size {
      {
        StoreH40(xself, dict, !(0usize), i);
      }
      i = i.wrapping_add(1 as (usize));
    }
  }
  if hasher_type == 41i32 {
    overlap = StoreLookaheadH41().wrapping_sub(1usize);
    i = 0usize;
    while i.wrapping_add(overlap) < size {
      {
        StoreH41(xself, dict, !(0usize), i);
      }
      i = i.wrapping_add(1 as (usize));
    }
  }
  if hasher_type == 42i32 {
    overlap = StoreLookaheadH42().wrapping_sub(1usize);
    i = 0usize;
    while i.wrapping_add(overlap) < size {
      {
        StoreH42(xself, dict, !(0usize), i);
      }
      i = i.wrapping_add(1 as (usize));
    }
  }
  if hasher_type == 54i32 {
    overlap = StoreLookaheadH54().wrapping_sub(1usize);
    i = 0usize;
    while i.wrapping_add(overlap) < size {
      {
        StoreH54(xself, dict, !(0usize), i);
      }
      i = i.wrapping_add(1 as (usize));
    }
  }
  if hasher_type == 10i32 {
    overlap = StoreLookaheadH10().wrapping_sub(1usize);
    i = 0usize;
    while i.wrapping_add(overlap) < size {
      {
        StoreH10(xself, dict, !(0usize), i);
      }
      i = i.wrapping_add(1 as (usize));
    }
  }
}


pub fn BrotliEncoderSetCustomDictionary(mut s: &mut [BrotliEncoderStateStruct],
                                        mut size: usize,
                                        mut dict: &[u8]) {
  let mut max_dict_size: usize = (1usize << (*s).params.lgwin).wrapping_sub(16usize);
  let mut dict_size: usize = size;
  let mut m: *mut MemoryManager = &mut (*s).memory_manager_;
  if EnsureInitialized(s) == 0 {
    return;
  }
  if dict_size == 0usize || (*s).params.quality == 0i32 || (*s).params.quality == 1i32 {
    return;
  }
  if size > max_dict_size {
    dict = dict[(size.wrapping_sub(max_dict_size) as (usize))..];
    dict_size = max_dict_size;
  }
  CopyInputToRingBuffer(s, dict_size, dict);
  (*s).last_flush_pos_ = dict_size;
  (*s).last_processed_pos_ = dict_size;
  if dict_size > 0usize {
    (*s).prev_byte_ = dict[(dict_size.wrapping_sub(1usize) as (usize))];
  }
  if dict_size > 1usize {
    (*s).prev_byte2_ = dict[(dict_size.wrapping_sub(2usize) as (usize))];
  }
  HasherPrependCustomDictionary(m, &mut (*s).hasher_, &mut (*s).params, dict_size, dict);
  if !(0i32 == 0) {}
}


pub fn BrotliEncoderMaxCompressedSize(mut input_size: usize) -> usize {
  let mut num_large_blocks: usize = input_size >> 24i32;
  let mut tail: usize = input_size.wrapping_sub(num_large_blocks << 24i32);
  let mut tail_overhead: usize = (if tail > (1i32 << 20i32) as (usize) {
                                    4i32
                                  } else {
                                    3i32
                                  }) as (usize);
  let mut overhead: usize = (2usize)
    .wrapping_add((4usize).wrapping_mul(num_large_blocks))
    .wrapping_add(tail_overhead)
    .wrapping_add(1usize);
  let mut result: usize = input_size.wrapping_add(overhead);
  if input_size == 0usize {
    return 1usize;
  }
  if result < input_size { 0usize } else { result }
}



pub struct BrotliDictionary {
  pub size_bits_by_length: [u8; 32],
  pub offsets_by_length: [u32; 32],
  pub data: [u8; 122784],
}

fn HashTypeLengthH2() -> usize {
  8usize
}

fn StitchToPreviousBlockH2(mut handle: &mut [u8],
                           mut num_bytes: usize,
                           mut position: usize,
                           mut ringbuffer: &[u8],
                           mut ringbuffer_mask: usize) {
  if num_bytes >= HashTypeLengthH2().wrapping_sub(1usize) && (position >= 3usize) {
    StoreH2(handle,
            ringbuffer,
            ringbuffer_mask,
            position.wrapping_sub(3usize));
    StoreH2(handle,
            ringbuffer,
            ringbuffer_mask,
            position.wrapping_sub(2usize));
    StoreH2(handle,
            ringbuffer,
            ringbuffer_mask,
            position.wrapping_sub(1usize));
  }
}

fn HashTypeLengthH3() -> usize {
  8usize
}

fn StitchToPreviousBlockH3(mut handle: &mut [u8],
                           mut num_bytes: usize,
                           mut position: usize,
                           mut ringbuffer: &[u8],
                           mut ringbuffer_mask: usize) {
  if num_bytes >= HashTypeLengthH3().wrapping_sub(1usize) && (position >= 3usize) {
    StoreH3(handle,
            ringbuffer,
            ringbuffer_mask,
            position.wrapping_sub(3usize));
    StoreH3(handle,
            ringbuffer,
            ringbuffer_mask,
            position.wrapping_sub(2usize));
    StoreH3(handle,
            ringbuffer,
            ringbuffer_mask,
            position.wrapping_sub(1usize));
  }
}

fn HashTypeLengthH4() -> usize {
  8usize
}

fn StitchToPreviousBlockH4(mut handle: &mut [u8],
                           mut num_bytes: usize,
                           mut position: usize,
                           mut ringbuffer: &[u8],
                           mut ringbuffer_mask: usize) {
  if num_bytes >= HashTypeLengthH4().wrapping_sub(1usize) && (position >= 3usize) {
    StoreH4(handle,
            ringbuffer,
            ringbuffer_mask,
            position.wrapping_sub(3usize));
    StoreH4(handle,
            ringbuffer,
            ringbuffer_mask,
            position.wrapping_sub(2usize));
    StoreH4(handle,
            ringbuffer,
            ringbuffer_mask,
            position.wrapping_sub(1usize));
  }
}

fn HashTypeLengthH5() -> usize {
  4usize
}

fn StitchToPreviousBlockH5(mut handle: &mut [u8],
                           mut num_bytes: usize,
                           mut position: usize,
                           mut ringbuffer: &[u8],
                           mut ringbuffer_mask: usize) {
  if num_bytes >= HashTypeLengthH5().wrapping_sub(1usize) && (position >= 3usize) {
    StoreH5(handle,
            ringbuffer,
            ringbuffer_mask,
            position.wrapping_sub(3usize));
    StoreH5(handle,
            ringbuffer,
            ringbuffer_mask,
            position.wrapping_sub(2usize));
    StoreH5(handle,
            ringbuffer,
            ringbuffer_mask,
            position.wrapping_sub(1usize));
  }
}

fn HashTypeLengthH6() -> usize {
  8usize
}

fn StitchToPreviousBlockH6(mut handle: &mut [u8],
                           mut num_bytes: usize,
                           mut position: usize,
                           mut ringbuffer: &[u8],
                           mut ringbuffer_mask: usize) {
  if num_bytes >= HashTypeLengthH6().wrapping_sub(1usize) && (position >= 3usize) {
    StoreH6(handle,
            ringbuffer,
            ringbuffer_mask,
            position.wrapping_sub(3usize));
    StoreH6(handle,
            ringbuffer,
            ringbuffer_mask,
            position.wrapping_sub(2usize));
    StoreH6(handle,
            ringbuffer,
            ringbuffer_mask,
            position.wrapping_sub(1usize));
  }
}

fn HashTypeLengthH40() -> usize {
  4usize
}

fn StitchToPreviousBlockH40(mut handle: &mut [u8],
                            mut num_bytes: usize,
                            mut position: usize,
                            mut ringbuffer: &[u8],
                            mut ring_buffer_mask: usize) {
  if num_bytes >= HashTypeLengthH40().wrapping_sub(1usize) && (position >= 3usize) {
    StoreH40(handle,
             ringbuffer,
             ring_buffer_mask,
             position.wrapping_sub(3usize));
    StoreH40(handle,
             ringbuffer,
             ring_buffer_mask,
             position.wrapping_sub(2usize));
    StoreH40(handle,
             ringbuffer,
             ring_buffer_mask,
             position.wrapping_sub(1usize));
  }
}

fn HashTypeLengthH41() -> usize {
  4usize
}

fn StitchToPreviousBlockH41(mut handle: &mut [u8],
                            mut num_bytes: usize,
                            mut position: usize,
                            mut ringbuffer: &[u8],
                            mut ring_buffer_mask: usize) {
  if num_bytes >= HashTypeLengthH41().wrapping_sub(1usize) && (position >= 3usize) {
    StoreH41(handle,
             ringbuffer,
             ring_buffer_mask,
             position.wrapping_sub(3usize));
    StoreH41(handle,
             ringbuffer,
             ring_buffer_mask,
             position.wrapping_sub(2usize));
    StoreH41(handle,
             ringbuffer,
             ring_buffer_mask,
             position.wrapping_sub(1usize));
  }
}

fn HashTypeLengthH42() -> usize {
  4usize
}

fn StitchToPreviousBlockH42(mut handle: &mut [u8],
                            mut num_bytes: usize,
                            mut position: usize,
                            mut ringbuffer: &[u8],
                            mut ring_buffer_mask: usize) {
  if num_bytes >= HashTypeLengthH42().wrapping_sub(1usize) && (position >= 3usize) {
    StoreH42(handle,
             ringbuffer,
             ring_buffer_mask,
             position.wrapping_sub(3usize));
    StoreH42(handle,
             ringbuffer,
             ring_buffer_mask,
             position.wrapping_sub(2usize));
    StoreH42(handle,
             ringbuffer,
             ring_buffer_mask,
             position.wrapping_sub(1usize));
  }
}

fn HashTypeLengthH54() -> usize {
  8usize
}

fn StitchToPreviousBlockH54(mut handle: &mut [u8],
                            mut num_bytes: usize,
                            mut position: usize,
                            mut ringbuffer: &[u8],
                            mut ringbuffer_mask: usize) {
  if num_bytes >= HashTypeLengthH54().wrapping_sub(1usize) && (position >= 3usize) {
    StoreH54(handle,
             ringbuffer,
             ringbuffer_mask,
             position.wrapping_sub(3usize));
    StoreH54(handle,
             ringbuffer,
             ringbuffer_mask,
             position.wrapping_sub(2usize));
    StoreH54(handle,
             ringbuffer,
             ringbuffer_mask,
             position.wrapping_sub(1usize));
  }
}

fn HashTypeLengthH10() -> usize {
  4usize
}

fn brotli_max_size_t(mut a: usize, mut b: usize) -> usize {
  if a > b { a } else { b }
}

fn StitchToPreviousBlockH10(mut handle: &mut [u8],
                            mut num_bytes: usize,
                            mut position: usize,
                            mut ringbuffer: &[u8],
                            mut ringbuffer_mask: usize) {
  let mut xself: *mut H10 = SelfH10(handle);
  if num_bytes >= HashTypeLengthH10().wrapping_sub(1usize) && (position >= 128usize) {
    let i_start: usize = position.wrapping_sub(128usize).wrapping_add(1usize);
    let i_end: usize = brotli_min_size_t(position, i_start.wrapping_add(num_bytes));
    let mut i: usize;
    i = i_start;
    while i < i_end {
      {
        let max_backward: usize =
          (*xself).window_mask_.wrapping_sub(brotli_max_size_t((16i32 - 1i32) as (usize),
                                                               position.wrapping_sub(i)));
        StoreAndFindMatchesH10(xself,
                               ringbuffer,
                               i,
                               ringbuffer_mask,
                               128usize,
                               max_backward,
                               0i32,
                               0i32);
      }
      i = i.wrapping_add(1 as (usize));
    }
  }
}

fn InitOrStitchToPreviousBlock(mut m: &mut [MemoryManager],
                               mut handle: &mut [*mut u8],
                               mut data: &[u8],
                               mut mask: usize,
                               mut params: &mut [BrotliEncoderParams],
                               mut position: usize,
                               mut input_size: usize,
                               mut is_last: i32) {
  let mut xself: *mut u8;
  HasherSetup(m, handle, params, data, position, input_size, is_last);
  if !(0i32 == 0) {
    return;
  }
  xself = *handle;
  let mut hasher_type: i32 = (*GetHasherCommon(xself)).params.type_;
  if hasher_type == 2i32 {
    StitchToPreviousBlockH2(xself, input_size, position, data, mask);
  }
  if hasher_type == 3i32 {
    StitchToPreviousBlockH3(xself, input_size, position, data, mask);
  }
  if hasher_type == 4i32 {
    StitchToPreviousBlockH4(xself, input_size, position, data, mask);
  }
  if hasher_type == 5i32 {
    StitchToPreviousBlockH5(xself, input_size, position, data, mask);
  }
  if hasher_type == 6i32 {
    StitchToPreviousBlockH6(xself, input_size, position, data, mask);
  }
  if hasher_type == 40i32 {
    StitchToPreviousBlockH40(xself, input_size, position, data, mask);
  }
  if hasher_type == 41i32 {
    StitchToPreviousBlockH41(xself, input_size, position, data, mask);
  }
  if hasher_type == 42i32 {
    StitchToPreviousBlockH42(xself, input_size, position, data, mask);
  }
  if hasher_type == 54i32 {
    StitchToPreviousBlockH54(xself, input_size, position, data, mask);
  }
  if hasher_type == 10i32 {
    StitchToPreviousBlockH10(xself, input_size, position, data, mask);
  }
}



pub struct Struct49 {
  pub cost: f32,
  pub next: u32,
  pub shortcut: u32,
}



pub struct ZopfliNode {
  pub length: u32,
  pub distance: u32,
  pub insert_length: u32,
  pub u: Struct49,
}

fn Log2FloorNonZero(mut n: usize) -> u32 {
  let mut result: u32 = 0u32;
  while {
          n = n >> 1i32;
          n
        } != 0 {
    result = result.wrapping_add(1 as (u32));
  }
  result
}

fn GetInsertLengthCode(mut insertlen: usize) -> u16 {
  if insertlen < 6usize {
    insertlen as (u16)
  } else if insertlen < 130usize {
    let mut nbits: u32 = Log2FloorNonZero(insertlen.wrapping_sub(2usize)).wrapping_sub(1u32);
    ((nbits << 1i32) as (usize))
      .wrapping_add(insertlen.wrapping_sub(2usize) >> nbits)
      .wrapping_add(2usize) as (u16)
  } else if insertlen < 2114usize {
    Log2FloorNonZero(insertlen.wrapping_sub(66usize)).wrapping_add(10u32) as (u16)
  } else if insertlen < 6210usize {
    21u32 as (u16)
  } else if insertlen < 22594usize {
    22u32 as (u16)
  } else {
    23u32 as (u16)
  }
}

fn GetCopyLengthCode(mut copylen: usize) -> u16 {
  if copylen < 10usize {
    copylen.wrapping_sub(2usize) as (u16)
  } else if copylen < 134usize {
    let mut nbits: u32 = Log2FloorNonZero(copylen.wrapping_sub(6usize)).wrapping_sub(1u32);
    ((nbits << 1i32) as (usize))
      .wrapping_add(copylen.wrapping_sub(6usize) >> nbits)
      .wrapping_add(4usize) as (u16)
  } else if copylen < 2118usize {
    Log2FloorNonZero(copylen.wrapping_sub(70usize)).wrapping_add(12u32) as (u16)
  } else {
    23u32 as (u16)
  }
}

fn CombineLengthCodes(mut inscode: u16, mut copycode: u16, mut use_last_distance: i32) -> u16 {
  let mut bits64: u16 = (copycode as (u32) & 0x7u32 | (inscode as (u32) & 0x7u32) << 3i32) as (u16);
  if use_last_distance != 0 && (inscode as (i32) < 8i32) && (copycode as (i32) < 16i32) {
    if copycode as (i32) < 8i32 {
      bits64
    } else {
      let mut s64: u16 = 64i32 as (u16);
      (bits64 as (i32) | s64 as (i32)) as (u16)
    }
  } else {
    let mut offset: i32 = 2i32 * ((copycode as (i32) >> 3i32) + 3i32 * (inscode as (i32) >> 3i32));
    offset = (offset << 5i32) + 0x40i32 + (0x520d40i32 >> offset & 0xc0i32);
    (offset as (u16) as (i32) | bits64 as (i32)) as (u16)
  }
}

fn GetLengthCode(mut insertlen: usize,
                 mut copylen: usize,
                 mut use_last_distance: i32,
                 mut code: &mut [u16]) {
  let mut inscode: u16 = GetInsertLengthCode(insertlen);
  let mut copycode: u16 = GetCopyLengthCode(copylen);
  *code = CombineLengthCodes(inscode, copycode, use_last_distance);
}

fn InitInsertCommand(mut xself: &mut Command, mut insertlen: usize) {
  (*xself).insert_len_ = insertlen as (u32);
  (*xself).copy_len_ = (4i32 << 24i32) as (u32);
  (*xself).dist_extra_ = 0u32;
  (*xself).dist_prefix_ = 16i32 as (u16);
  GetLengthCode(insertlen, 4usize, 0i32, &mut (*xself).cmd_prefix_);
}

fn BROTLI_UNALIGNED_STORE64(mut p: &mut [::std::os::raw::c_void], mut v: usize) {
  memcpy(p, &mut v, ::std::mem::size_of::<usize>());
}

fn BrotliWriteBits(mut n_bits: usize,
                   mut bits: usize,
                   mut pos: &mut [usize],
                   mut array: &mut [u8]) {
  let mut p: *mut u8 = &mut array[((*pos >> 3i32) as (usize))];
  let mut v: usize = *p as (usize);
  0i32;
  0i32;
  v = v | bits << (*pos & 7usize);
  BROTLI_UNALIGNED_STORE64(p, v);
  *pos = (*pos).wrapping_add(n_bits);
}

fn FastLog2(mut v: usize) -> f64 {
  if v < ::std::mem::size_of::<[f32; 256]>().wrapping_div(::std::mem::size_of::<f32>()) {
    return kLog2Table[v] as (f64);
  }
  log2(v as (f64))
}

fn ShannonEntropy(mut population: &[u32], mut size: usize, mut total: &mut usize) -> f64 {
  let mut sum: usize = 0usize;
  let mut retval: f64 = 0i32 as (f64);
  let mut population_end: *const u32 = population[(size as (usize))..];
  let mut p: usize;
  let mut odd_number_of_elements_left: i32 = 0i32;
  if size & 1usize != 0 {
    odd_number_of_elements_left = 1i32;
  }
  while population < population_end {
    if odd_number_of_elements_left == 0 {
      p = *{
             let _old = population;
             population = population[(1 as (usize))..];
             _old
           } as (usize);
      sum = sum.wrapping_add(p);
      retval = retval - p as (f64) * FastLog2(p);
    }
    odd_number_of_elements_left = 0i32;
    p = *{
           let _old = population;
           population = population[(1 as (usize))..];
           _old
         } as (usize);
    sum = sum.wrapping_add(p);
    retval = retval - p as (f64) * FastLog2(p);
  }
  if sum != 0 {
    retval = retval + sum as (f64) * FastLog2(sum);
  }
  *total = sum;
  retval
}

fn BitsEntropy(mut population: &[u32], mut size: usize) -> f64 {
  let mut sum: usize;
  let mut retval: f64 = ShannonEntropy(population, size, &mut sum);
  if retval < sum as (f64) {
    retval = sum as (f64);
  }
  retval
}

fn ShouldCompress(mut data: &[u8],
                  mask: usize,
                  last_flush_pos: usize,
                  bytes: usize,
                  num_literals: usize,
                  num_commands: usize)
                  -> i32 {
  if num_commands < (bytes >> 8i32).wrapping_add(2usize) {
    if num_literals as (f64) > 0.99f64 * bytes as (f64) {
      let mut literal_histo: [u32; 256] =
        [0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
         0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
         0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
         0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
         0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
         0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
         0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
         0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
         0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
         0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
         0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
         0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
         0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
         0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
         0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
         0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
         0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
         0u32];
      static kSampleRate: u32 = 13u32;
      static kMinEntropy: f64 = 7.92f64;
      let bit_cost_threshold: f64 = bytes as (f64) * kMinEntropy / kSampleRate as (f64);
      let mut t: usize = bytes.wrapping_add(kSampleRate as (usize))
        .wrapping_sub(1usize)
        .wrapping_div(kSampleRate as (usize));
      let mut pos: u32 = last_flush_pos as (u32);
      let mut i: usize;
      i = 0usize;
      while i < t {
        {
          {
            let _rhs = 1;
            let _lhs = &mut literal_histo[data[((pos as (usize) & mask) as (usize))] as (usize)];
            *_lhs = (*_lhs).wrapping_add(_rhs as (u32));
          }
          pos = pos.wrapping_add(kSampleRate);
        }
        i = i.wrapping_add(1 as (usize));
      }
      if BitsEntropy(literal_histo.as_mut_ptr(), 256usize) > bit_cost_threshold {
        return 0i32;
      }
    }
  }
  1i32
}


#[repr(i32)]
pub enum ContextType {
  CONTEXT_LSB6 = 0i32,
  CONTEXT_MSB6 = 1i32,
  CONTEXT_UTF8 = 2i32,
  CONTEXT_SIGNED = 3i32,
}



pub struct BlockSplit {
  pub num_types: usize,
  pub num_blocks: usize,
  pub types: *mut u8,
  pub lengths: *mut u32,
  pub types_alloc_size: usize,
  pub lengths_alloc_size: usize,
}



pub struct HistogramLiteral {
  pub data_: [u32; 256],
  pub total_count_: usize,
  pub bit_cost_: f64,
}



pub struct HistogramCommand {
  pub data_: [u32; 704],
  pub total_count_: usize,
  pub bit_cost_: f64,
}



pub struct HistogramDistance {
  pub data_: [u32; 520],
  pub total_count_: usize,
  pub bit_cost_: f64,
}



pub struct MetaBlockSplit {
  pub literal_split: BlockSplit,
  pub command_split: BlockSplit,
  pub distance_split: BlockSplit,
  pub literal_context_map: *mut u32,
  pub literal_context_map_size: usize,
  pub distance_context_map: *mut u32,
  pub distance_context_map_size: usize,
  pub literal_histograms: *mut HistogramLiteral,
  pub literal_histograms_size: usize,
  pub command_histograms: *mut HistogramCommand,
  pub command_histograms_size: usize,
  pub distance_histograms: *mut HistogramDistance,
  pub distance_histograms_size: usize,
}

fn InitMetaBlockSplit(mut mb: &mut [MetaBlockSplit]) {
  BrotliInitBlockSplit(&mut (*mb).literal_split);
  BrotliInitBlockSplit(&mut (*mb).command_split);
  BrotliInitBlockSplit(&mut (*mb).distance_split);
  (*mb).literal_context_map = 0i32;
  (*mb).literal_context_map_size = 0usize;
  (*mb).distance_context_map = 0i32;
  (*mb).distance_context_map_size = 0usize;
  (*mb).literal_histograms = 0i32;
  (*mb).literal_histograms_size = 0usize;
  (*mb).command_histograms = 0i32;
  (*mb).command_histograms_size = 0usize;
  (*mb).distance_histograms = 0i32;
  (*mb).distance_histograms_size = 0usize;
}

fn DestroyMetaBlockSplit(mut m: &mut [MemoryManager], mut mb: &mut [MetaBlockSplit]) {
  BrotliDestroyBlockSplit(m, &mut (*mb).literal_split);
  BrotliDestroyBlockSplit(m, &mut (*mb).command_split);
  BrotliDestroyBlockSplit(m, &mut (*mb).distance_split);
  {
    BrotliFree(m, (*mb).literal_context_map);
    (*mb).literal_context_map = 0i32;
  }
  {
    BrotliFree(m, (*mb).distance_context_map);
    (*mb).distance_context_map = 0i32;
  }
  {
    BrotliFree(m, (*mb).literal_histograms);
    (*mb).literal_histograms = 0i32;
  }
  {
    BrotliFree(m, (*mb).command_histograms);
    (*mb).command_histograms = 0i32;
  }
  {
    BrotliFree(m, (*mb).distance_histograms);
    (*mb).distance_histograms = 0i32;
  }
}

fn BrotliCompressBufferQuality10(mut lgwin: i32,
                                 mut input_size: usize,
                                 mut input_buffer: &[u8],
                                 mut encoded_size: &mut [usize],
                                 mut encoded_buffer: &mut [u8])
                                 -> i32 {
  let mut memory_manager: MemoryManager;
  let mut m: *mut MemoryManager = &mut memory_manager;
  let mask: usize = !(0usize) >> 1i32;
  let max_backward_limit: usize = (1usize << lgwin).wrapping_sub(16usize);
  let mut dist_cache: [i32; 4] = [4i32, 11i32, 15i32, 16i32];
  let mut saved_dist_cache: [i32; 4] = [4i32, 11i32, 15i32, 16i32];
  let mut ok: i32 = 1i32;
  let max_out_size: usize = *encoded_size;
  let mut total_out_size: usize = 0usize;
  let mut last_byte: u8;
  let mut last_byte_bits: u8;
  let mut hasher: *mut u8 = 0i32;
  let hasher_eff_size: usize = brotli_min_size_t(input_size,
                                                 max_backward_limit.wrapping_add(16usize));
  let mut params: BrotliEncoderParams;
  let mut dictionary: *const BrotliDictionary = BrotliGetDictionary();
  let lgmetablock: i32 = brotli_min_int(24i32, lgwin + 1i32);
  let mut max_block_size: usize;
  let max_metablock_size: usize = 1usize << lgmetablock;
  let max_literals_per_metablock: usize = max_metablock_size.wrapping_div(8usize);
  let max_commands_per_metablock: usize = max_metablock_size.wrapping_div(8usize);
  let mut metablock_start: usize = 0usize;
  let mut prev_byte: u8 = 0i32 as (u8);
  let mut prev_byte2: u8 = 0i32 as (u8);
  BrotliEncoderInitParams(&mut params);
  params.quality = 10i32;
  params.lgwin = lgwin;
  SanitizeParams(&mut params);
  params.lgblock = ComputeLgBlock(&mut params);
  max_block_size = 1usize << params.lgblock;
  BrotliInitMemoryManager(m,
                          0i32 as
                          (fn(*mut ::std::os::raw::c_void, usize) -> *mut ::std::os::raw::c_void),
                          0i32 as (fn(*mut ::std::os::raw::c_void, *mut ::std::os::raw::c_void)),
                          0i32);
  0i32;
  EncodeWindowBits(lgwin, &mut last_byte, &mut last_byte_bits);
  InitOrStitchToPreviousBlock(m,
                              &mut hasher,
                              input_buffer,
                              mask,
                              &mut params,
                              0usize,
                              hasher_eff_size,
                              1i32);
  if !(0i32 == 0) {
    BrotliWipeOutMemoryManager(m);
    return 0i32;
  }
  while ok != 0 && (metablock_start < input_size) {
    let metablock_end: usize = brotli_min_size_t(input_size,
                                                 metablock_start.wrapping_add(max_metablock_size));
    let expected_num_commands: usize =
      metablock_end.wrapping_sub(metablock_start).wrapping_div(12usize).wrapping_add(16usize);
    let mut commands: *mut Command = 0i32;
    let mut num_commands: usize = 0usize;
    let mut last_insert_len: usize = 0usize;
    let mut num_literals: usize = 0usize;
    let mut metablock_size: usize = 0usize;
    let mut cmd_alloc_size: usize = 0usize;
    let mut is_last: i32;
    let mut storage: *mut u8;
    let mut storage_ix: usize;
    let mut block_start: usize;
    block_start = metablock_start;
    while block_start < metablock_end {
      let mut block_size: usize = brotli_min_size_t(metablock_end.wrapping_sub(block_start),
                                                    max_block_size);
      let mut nodes: *mut ZopfliNode = if block_size.wrapping_add(1usize) != 0 {
        BrotliAllocate(m,
                       block_size.wrapping_add(1usize)
                         .wrapping_mul(::std::mem::size_of::<ZopfliNode>()))
      } else {
        0i32
      };
      let mut path_size: usize;
      let mut new_cmd_alloc_size: usize;
      if !(0i32 == 0) {
        BrotliWipeOutMemoryManager(m);
        return 0i32;
      }
      BrotliInitZopfliNodes(nodes, block_size.wrapping_add(1usize));
      StitchToPreviousBlockH10(hasher, block_size, block_start, input_buffer, mask);
      path_size = BrotliZopfliComputeShortestPath(m,
                                                  dictionary,
                                                  block_size,
                                                  block_start,
                                                  input_buffer,
                                                  mask,
                                                  &mut params,
                                                  max_backward_limit,
                                                  dist_cache.as_mut_ptr(),
                                                  hasher,
                                                  nodes);
      if !(0i32 == 0) {
        BrotliWipeOutMemoryManager(m);
        return 0i32;
      }
      new_cmd_alloc_size = brotli_max_size_t(expected_num_commands,
                                             num_commands.wrapping_add(path_size)
                                               .wrapping_add(1usize));
      if cmd_alloc_size != new_cmd_alloc_size {
        let mut new_commands: *mut Command = if new_cmd_alloc_size != 0 {
          BrotliAllocate(m,
                         new_cmd_alloc_size.wrapping_mul(::std::mem::size_of::<Command>()))
        } else {
          0i32
        };
        if !(0i32 == 0) {
          BrotliWipeOutMemoryManager(m);
          return 0i32;
        }
        cmd_alloc_size = new_cmd_alloc_size;
        if !commands.is_null() {
          memcpy(new_commands,
                 commands,
                 ::std::mem::size_of::<Command>().wrapping_mul(num_commands));
          {
            BrotliFree(m, commands);
            commands = 0i32;
          }
        }
        commands = new_commands;
      }
      BrotliZopfliCreateCommands(block_size,
                                 block_start,
                                 max_backward_limit,
                                 &mut nodes[(0usize)],
                                 dist_cache.as_mut_ptr(),
                                 &mut last_insert_len,
                                 &mut commands[(num_commands as (usize))],
                                 &mut num_literals);
      num_commands = num_commands.wrapping_add(path_size);
      block_start = block_start.wrapping_add(block_size);
      metablock_size = metablock_size.wrapping_add(block_size);
      {
        BrotliFree(m, nodes);
        nodes = 0i32;
      }
      if num_literals > max_literals_per_metablock || num_commands > max_commands_per_metablock {
        {
          break;
        }
      }
    }
    if last_insert_len > 0usize {
      InitInsertCommand(&mut commands[({
                                let _old = num_commands;
                                num_commands = num_commands.wrapping_add(1 as (usize));
                                _old
                              } as (usize))],
                        last_insert_len);
      num_literals = num_literals.wrapping_add(last_insert_len);
    }
    is_last = if !!(metablock_start.wrapping_add(metablock_size) == input_size) {
      1i32
    } else {
      0i32
    };
    storage = 0i32;
    storage_ix = last_byte_bits as (usize);
    if metablock_size == 0usize {
      storage = if 16i32 != 0 {
        BrotliAllocate(m, (16usize).wrapping_mul(::std::mem::size_of::<u8>()))
      } else {
        0i32
      };
      if !(0i32 == 0) {
        BrotliWipeOutMemoryManager(m);
        return 0i32;
      }
      storage[(0usize)] = last_byte;
      BrotliWriteBits(2usize, 3usize, &mut storage_ix, storage);
      storage_ix = storage_ix.wrapping_add(7u32 as (usize)) & !7u32 as (usize);
    } else if ShouldCompress(input_buffer,
                             mask,
                             metablock_start,
                             metablock_size,
                             num_literals,
                             num_commands) == 0 {
      memcpy(dist_cache.as_mut_ptr(),
             saved_dist_cache.as_mut_ptr(),
             (4usize).wrapping_mul(::std::mem::size_of::<i32>()));
      storage = if metablock_size.wrapping_add(16usize) != 0 {
        BrotliAllocate(m,
                       metablock_size.wrapping_add(16usize)
                         .wrapping_mul(::std::mem::size_of::<u8>()))
      } else {
        0i32
      };
      if !(0i32 == 0) {
        BrotliWipeOutMemoryManager(m);
        return 0i32;
      }
      storage[(0usize)] = last_byte;
      BrotliStoreUncompressedMetaBlock(is_last,
                                       input_buffer,
                                       metablock_start,
                                       mask,
                                       metablock_size,
                                       &mut storage_ix,
                                       storage);
    } else {
      let mut num_direct_distance_codes: u32 = 0u32;
      let mut distance_postfix_bits: u32 = 0u32;
      let mut literal_context_mode: ContextType = ContextType::CONTEXT_UTF8;
      let mut mb: MetaBlockSplit;
      InitMetaBlockSplit(&mut mb);
      if BrotliIsMostlyUTF8(input_buffer,
                            metablock_start,
                            mask,
                            metablock_size,
                            kMinUTF8Ratio) == 0 {
        literal_context_mode = ContextType::CONTEXT_SIGNED;
      }
      BrotliBuildMetaBlock(m,
                           input_buffer,
                           metablock_start,
                           mask,
                           &mut params,
                           prev_byte,
                           prev_byte2,
                           commands,
                           num_commands,
                           literal_context_mode,
                           &mut mb);
      if !(0i32 == 0) {
        BrotliWipeOutMemoryManager(m);
        return 0i32;
      }
      BrotliOptimizeHistograms(num_direct_distance_codes as (usize),
                               distance_postfix_bits as (usize),
                               &mut mb);
      storage = if (2usize).wrapping_mul(metablock_size).wrapping_add(502usize) != 0 {
        BrotliAllocate(m,
                       (2usize)
                         .wrapping_mul(metablock_size)
                         .wrapping_add(502usize)
                         .wrapping_mul(::std::mem::size_of::<u8>()))
      } else {
        0i32
      };
      if !(0i32 == 0) {
        BrotliWipeOutMemoryManager(m);
        return 0i32;
      }
      storage[(0usize)] = last_byte;
      BrotliStoreMetaBlock(m,
                           input_buffer,
                           metablock_start,
                           metablock_size,
                           mask,
                           prev_byte,
                           prev_byte2,
                           is_last,
                           num_direct_distance_codes,
                           distance_postfix_bits,
                           literal_context_mode,
                           commands,
                           num_commands,
                           &mut mb,
                           &mut storage_ix,
                           storage);
      if !(0i32 == 0) {
        BrotliWipeOutMemoryManager(m);
        return 0i32;
      }
      if metablock_size.wrapping_add(4usize) < storage_ix >> 3i32 {
        memcpy(dist_cache.as_mut_ptr(),
               saved_dist_cache.as_mut_ptr(),
               (4usize).wrapping_mul(::std::mem::size_of::<i32>()));
        storage[(0usize)] = last_byte;
        storage_ix = last_byte_bits as (usize);
        BrotliStoreUncompressedMetaBlock(is_last,
                                         input_buffer,
                                         metablock_start,
                                         mask,
                                         metablock_size,
                                         &mut storage_ix,
                                         storage);
      }
      DestroyMetaBlockSplit(m, &mut mb);
    }
    last_byte = storage[((storage_ix >> 3i32) as (usize))];
    last_byte_bits = (storage_ix & 7u32 as (usize)) as (u8);
    metablock_start = metablock_start.wrapping_add(metablock_size);
    prev_byte = input_buffer[(metablock_start.wrapping_sub(1usize) as (usize))];
    prev_byte2 = input_buffer[(metablock_start.wrapping_sub(2usize) as (usize))];
    memcpy(saved_dist_cache.as_mut_ptr(),
           dist_cache.as_mut_ptr(),
           (4usize).wrapping_mul(::std::mem::size_of::<i32>()));
    {
      let out_size: usize = storage_ix >> 3i32;
      total_out_size = total_out_size.wrapping_add(out_size);
      if total_out_size <= max_out_size {
        memcpy(encoded_buffer, storage, out_size);
        encoded_buffer = encoded_buffer[(out_size as (usize))..];
      } else {
        ok = 0i32;
      }
    }
    {
      BrotliFree(m, storage);
      storage = 0i32;
    }
    {
      BrotliFree(m, commands);
      commands = 0i32;
    }
  }
  *encoded_size = total_out_size;
  DestroyHasher(m, &mut hasher);
  return ok;
  BrotliWipeOutMemoryManager(m);
  0i32
}


#[repr(i32)]
pub enum BrotliEncoderOperation {
  BROTLI_OPERATION_PROCESS = 0i32,
  BROTLI_OPERATION_FLUSH = 1i32,
  BROTLI_OPERATION_FINISH = 2i32,
  BROTLI_OPERATION_EMIT_METADATA = 3i32,
}

fn MakeUncompressedStream(mut input: &[u8], mut input_size: usize, mut output: &mut [u8]) -> usize {
  let mut size: usize = input_size;
  let mut result: usize = 0usize;
  let mut offset: usize = 0usize;
  if input_size == 0usize {
    output[(0usize)] = 6i32 as (u8);
    return 1usize;
  }
  output[({
     let _old = result;
     result = result.wrapping_add(1 as (usize));
     _old
   } as (usize))] = 0x21i32 as (u8);
  output[({
     let _old = result;
     result = result.wrapping_add(1 as (usize));
     _old
   } as (usize))] = 0x3i32 as (u8);
  while size > 0usize {
    let mut nibbles: u32 = 0u32;
    let mut chunk_size: u32;
    let mut bits: u32;
    chunk_size = if size > (1u32 << 24i32) as (usize) {
      1u32 << 24i32
    } else {
      size as (u32)
    };
    if chunk_size > 1u32 << 16i32 {
      nibbles = if chunk_size > 1u32 << 20i32 {
        2i32
      } else {
        1i32
      } as (u32);
    }
    bits = nibbles << 1i32 | chunk_size.wrapping_sub(1u32) << 3i32 |
           1u32 << (19u32).wrapping_add((4u32).wrapping_mul(nibbles));
    output[({
       let _old = result;
       result = result.wrapping_add(1 as (usize));
       _old
     } as (usize))] = bits as (u8);
    output[({
       let _old = result;
       result = result.wrapping_add(1 as (usize));
       _old
     } as (usize))] = (bits >> 8i32) as (u8);
    output[({
       let _old = result;
       result = result.wrapping_add(1 as (usize));
       _old
     } as (usize))] = (bits >> 16i32) as (u8);
    if nibbles == 2u32 {
      output[({
         let _old = result;
         result = result.wrapping_add(1 as (usize));
         _old
       } as (usize))] = (bits >> 24i32) as (u8);
    }
    memcpy(&mut output[(result as (usize))],
           &input[(offset as (usize))],
           chunk_size as (usize));
    result = result.wrapping_add(chunk_size as (usize));
    offset = offset.wrapping_add(chunk_size as (usize));
    size = size.wrapping_sub(chunk_size as (usize));
  }
  output[({
     let _old = result;
     result = result.wrapping_add(1 as (usize));
     _old
   } as (usize))] = 3i32 as (u8);
  result
}


pub fn BrotliEncoderCompress(mut quality: i32,
                             mut lgwin: i32,
                             mut mode: BrotliEncoderMode,
                             mut input_size: usize,
                             mut input_buffer: &[u8],
                             mut encoded_size: &mut [usize],
                             mut encoded_buffer: &mut [u8])
                             -> i32 {
  let mut s: *mut BrotliEncoderStateStruct;
  let mut out_size: usize = *encoded_size;
  let mut input_start: *const u8 = input_buffer;
  let mut output_start: *mut u8 = encoded_buffer;
  let mut max_out_size: usize = BrotliEncoderMaxCompressedSize(input_size);
  if out_size == 0usize {
    return 0i32;
  }
  if input_size == 0usize {
    *encoded_size = 1usize;
    *encoded_buffer = 6i32 as (u8);
    return 1i32;
  }
  let mut is_fallback: i32 = 0i32;
  if quality == 10i32 {
    let lg_win: i32 = brotli_min_int(24i32, brotli_max_int(16i32, lgwin));
    let mut ok: i32 = BrotliCompressBufferQuality10(lg_win,
                                                    input_size,
                                                    input_buffer,
                                                    encoded_size,
                                                    encoded_buffer);
    if ok == 0 || max_out_size != 0 && (*encoded_size > max_out_size) {
      is_fallback = 1i32;
    } else {
      return 1i32;
    }
  }
  if is_fallback == 0 {
    s = BrotliEncoderCreateInstance(0i32 as
                                    (fn(*mut ::std::os::raw::c_void, usize)
                                        -> *mut ::std::os::raw::c_void),
                                    0i32 as
                                    (fn(*mut ::std::os::raw::c_void, *mut ::std::os::raw::c_void)),
                                    0i32);
    if s.is_null() {
      return 0i32;
    } else {
      let mut available_in: usize = input_size;
      let mut next_in: *const u8 = input_buffer;
      let mut available_out: usize = *encoded_size;
      let mut next_out: *mut u8 = encoded_buffer;
      let mut total_out: usize = 0usize;
      let mut result: i32 = 0i32;
      BrotliEncoderSetParameter(s,
                                BrotliEncoderParameter::BROTLI_PARAM_QUALITY,
                                quality as (u32));
      BrotliEncoderSetParameter(s,
                                BrotliEncoderParameter::BROTLI_PARAM_LGWIN,
                                lgwin as (u32));
      BrotliEncoderSetParameter(s, BrotliEncoderParameter::BROTLI_PARAM_MODE, mode as (u32));
      BrotliEncoderSetParameter(s,
                                BrotliEncoderParameter::BROTLI_PARAM_SIZE_HINT,
                                input_size as (u32));
      result = BrotliEncoderCompressStream(s,
                                           BrotliEncoderOperation::BROTLI_OPERATION_FINISH,
                                           &mut available_in,
                                           &mut next_in,
                                           &mut available_out,
                                           &mut next_out,
                                           &mut total_out);
      if BrotliEncoderIsFinished(s) == 0 {
        result = 0i32;
      }
      *encoded_size = total_out;
      BrotliEncoderDestroyInstance(s);
      if result == 0 || max_out_size != 0 && (*encoded_size > max_out_size) {
        is_fallback = 1i32;
      } else {
        return 1i32;
      }
    }
  }
  *encoded_size = 0usize;
  if max_out_size == 0 {
    return 0i32;
  }
  if out_size >= max_out_size {
    *encoded_size = MakeUncompressedStream(input_start, input_size, output_start);
    return 1i32;
  }
  0i32
}

fn UnprocessedInputSize(mut s: &mut [BrotliEncoderStateStruct]) -> usize {
  (*s).input_pos_.wrapping_sub((*s).last_processed_pos_)
}

fn UpdateSizeHint(mut s: &mut [BrotliEncoderStateStruct], mut available_in: usize) {
  if (*s).params.size_hint == 0usize {
    let mut delta: usize = UnprocessedInputSize(s);
    let mut tail: usize = available_in;
    let mut limit: u32 = 1u32 << 30i32;
    let mut total: u32;
    if delta >= limit as (usize) || tail >= limit as (usize) ||
       delta.wrapping_add(tail) >= limit as (usize) {
      total = limit;
    } else {
      total = delta.wrapping_add(tail) as (u32);
    }
    (*s).params.size_hint = total as (usize);
  }
}

fn InjectBytePaddingBlock(mut s: &mut [BrotliEncoderStateStruct]) {
  let mut seal: u32 = (*s).last_byte_ as (u32);
  let mut seal_bits: usize = (*s).last_byte_bits_ as (usize);
  let mut destination: *mut u8;
  (*s).last_byte_ = 0i32 as (u8);
  (*s).last_byte_bits_ = 0i32 as (u8);
  seal = seal | 0x6u32 << seal_bits;
  seal_bits = seal_bits.wrapping_add(6usize);
  if !(*s).next_out_.is_null() {
    destination = (*s).next_out_[((*s).available_out_ as (usize))..];
  } else {
    destination = (*s).tiny_buf_.u8.as_mut_ptr();
    (*s).next_out_ = destination;
  }
  destination[(0usize)] = seal as (u8);
  if seal_bits > 8usize {
    destination[(1usize)] = (seal >> 8i32) as (u8);
  }
  (*s).available_out_ = (*s).available_out_.wrapping_add(seal_bits.wrapping_add(7usize) >> 3i32);
}

fn InjectFlushOrPushOutput(mut s: &mut [BrotliEncoderStateStruct],
                           mut available_out: &mut [usize],
                           mut next_out: &mut [*mut u8],
                           mut total_out: &mut usize)
                           -> i32 {
  if (*s).stream_state_ as (i32) ==
     BrotliEncoderStreamState::BROTLI_STREAM_FLUSH_REQUESTED as (i32) &&
     ((*s).last_byte_bits_ as (i32) != 0i32) {
    InjectBytePaddingBlock(s);
    return 1i32;
  }
  if (*s).available_out_ != 0usize && (*available_out != 0usize) {
    let mut copy_output_size: usize = brotli_min_size_t((*s).available_out_, *available_out);
    memcpy(*next_out, (*s).next_out_, copy_output_size);
    *next_out = (*next_out).offset(copy_output_size as (isize));
    *available_out = (*available_out).wrapping_sub(copy_output_size);
    (*s).next_out_ = (*s).next_out_[(copy_output_size as (usize))..];
    (*s).available_out_ = (*s).available_out_.wrapping_sub(copy_output_size);
    (*s).total_out_ = (*s).total_out_.wrapping_add(copy_output_size);
    if !total_out.is_null() {
      *total_out = (*s).total_out_;
    }
    return 1i32;
  }
  0i32
}

fn WrapPosition(mut position: usize) -> u32 {
  let mut result: u32 = position as (u32);
  let mut gb: usize = position >> 30i32;
  if gb > 2usize {
    result = result & (1u32 << 30i32).wrapping_sub(1u32) |
             ((gb.wrapping_sub(1usize) & 1usize) as (u32)).wrapping_add(1u32) << 30i32;
  }
  result
}

fn InputBlockSize(mut s: &mut [BrotliEncoderStateStruct]) -> usize {
  if EnsureInitialized(s) == 0 {
    return 0usize;
  }
  1usize << (*s).params.lgblock
}

fn GetBrotliStorage(mut s: &mut [BrotliEncoderStateStruct], mut size: usize) -> *mut u8 {
  let mut m: *mut MemoryManager = &mut (*s).memory_manager_;
  if (*s).storage_size_ < size {
    {
      BrotliFree(m, (*s).storage_);
      (*s).storage_ = 0i32;
    }
    (*s).storage_ = if size != 0 {
      BrotliAllocate(m, size.wrapping_mul(::std::mem::size_of::<u8>()))
    } else {
      0i32
    };
    if !(0i32 == 0) {
      return 0i32;
    }
    (*s).storage_size_ = size;
  }
  (*s).storage_
}

fn MaxHashTableSize(mut quality: i32) -> usize {
  (if quality == 0i32 {
     1i32 << 15i32
   } else {
     1i32 << 17i32
   }) as (usize)
}

fn HashTableSize(mut max_table_size: usize, mut input_size: usize) -> usize {
  let mut htsize: usize = 256usize;
  while htsize < max_table_size && (htsize < input_size) {
    htsize = htsize << 1i32;
  }
  htsize
}

fn GetHashTable(mut s: &mut [BrotliEncoderStateStruct],
                mut quality: i32,
                mut input_size: usize,
                mut table_size: &mut [usize])
                -> *mut i32 {
  let mut m: *mut MemoryManager = &mut (*s).memory_manager_;
  let max_table_size: usize = MaxHashTableSize(quality);
  let mut htsize: usize = HashTableSize(max_table_size, input_size);
  let mut table: *mut i32;
  0i32;
  if quality == 0i32 {
    if htsize & 0xaaaaausize == 0usize {
      htsize = htsize << 1i32;
    }
  }
  if htsize <= ::std::mem::size_of::<[i32; 1024]>().wrapping_div(::std::mem::size_of::<i32>()) {
    table = (*s).small_table_.as_mut_ptr();
  } else {
    if htsize > (*s).large_table_size_ {
      (*s).large_table_size_ = htsize;
      {
        BrotliFree(m, (*s).large_table_);
        (*s).large_table_ = 0i32;
      }
      (*s).large_table_ = if htsize != 0 {
        BrotliAllocate(m, htsize.wrapping_mul(::std::mem::size_of::<i32>()))
      } else {
        0i32
      };
      if !(0i32 == 0) {
        return 0i32;
      }
    }
    table = (*s).large_table_;
  }
  *table_size = htsize;
  memset(table,
         0i32,
         htsize.wrapping_mul(::std::mem::size_of::<i32>()));
  table
}

fn UpdateLastProcessedPos(mut s: &mut [BrotliEncoderStateStruct]) -> i32 {
  let mut wrapped_last_processed_pos: u32 = WrapPosition((*s).last_processed_pos_);
  let mut wrapped_input_pos: u32 = WrapPosition((*s).input_pos_);
  (*s).last_processed_pos_ = (*s).input_pos_;
  if !!(wrapped_input_pos < wrapped_last_processed_pos) {
    1i32
  } else {
    0i32
  }
}

fn MaxMetablockSize(mut params: &[BrotliEncoderParams]) -> usize {
  let mut bits: i32 = brotli_min_int(ComputeRbBits(params), 24i32);
  1usize << bits
}

fn CommandCopyLen(mut xself: &Command) -> u32 {
  (*xself).copy_len_ & 0xffffffu32
}

fn PrefixEncodeCopyDistance(mut distance_code: usize,
                            mut num_direct_codes: usize,
                            mut postfix_bits: usize,
                            mut code: &mut [u16],
                            mut extra_bits: &mut [u32]) {
  if distance_code < (16usize).wrapping_add(num_direct_codes) {
    *code = distance_code as (u16);
    *extra_bits = 0u32;
  } else {
    let mut dist: usize =
      (1usize << postfix_bits.wrapping_add(2u32 as (usize)))
        .wrapping_add(distance_code.wrapping_sub(16usize).wrapping_sub(num_direct_codes));
    let mut bucket: usize = Log2FloorNonZero(dist).wrapping_sub(1u32) as (usize);
    let mut postfix_mask: usize = (1u32 << postfix_bits).wrapping_sub(1u32) as (usize);
    let mut postfix: usize = dist & postfix_mask;
    let mut prefix: usize = dist >> bucket & 1usize;
    let mut offset: usize = (2usize).wrapping_add(prefix) << bucket;
    let mut nbits: usize = bucket.wrapping_sub(postfix_bits);
    *code = (16usize)
      .wrapping_add(num_direct_codes)
      .wrapping_add((2usize).wrapping_mul(nbits.wrapping_sub(1usize)).wrapping_add(prefix) <<
                    postfix_bits)
      .wrapping_add(postfix) as (u16);
    *extra_bits = (nbits << 24i32 | dist.wrapping_sub(offset) >> postfix_bits) as (u32);
  }
}

fn CommandRestoreDistanceCode(mut xself: &Command) -> u32 {
  if (*xself).dist_prefix_ as (i32) < 16i32 {
    (*xself).dist_prefix_ as (u32)
  } else {
    let mut nbits: u32 = (*xself).dist_extra_ >> 24i32;
    let mut extra: u32 = (*xself).dist_extra_ & 0xffffffu32;
    let mut prefix: u32 = ((*xself).dist_prefix_ as (u32))
      .wrapping_add(4u32)
      .wrapping_sub(16u32)
      .wrapping_sub(2u32.wrapping_mul(nbits));
    (prefix << nbits).wrapping_add(extra).wrapping_add(16u32).wrapping_sub(4u32)
  }
}

fn RecomputeDistancePrefixes(mut cmds: &mut [Command],
                             mut num_commands: usize,
                             mut num_direct_distance_codes: u32,
                             mut distance_postfix_bits: u32) {
  let mut i: usize;
  if num_direct_distance_codes == 0u32 && (distance_postfix_bits == 0u32) {
    return;
  }
  i = 0usize;
  while i < num_commands {
    {
      let mut cmd: *mut Command = &mut cmds[(i as (usize))];
      if CommandCopyLen(cmd) != 0 && ((*cmd).cmd_prefix_ as (i32) >= 128i32) {
        PrefixEncodeCopyDistance(CommandRestoreDistanceCode(cmd) as (usize),
                                 num_direct_distance_codes as (usize),
                                 distance_postfix_bits as (usize),
                                 &mut (*cmd).dist_prefix_,
                                 &mut (*cmd).dist_extra_);
      }
    }
    i = i.wrapping_add(1 as (usize));
  }
}

fn ChooseContextMap(mut quality: i32,
                    mut bigram_histo: &mut [u32],
                    mut num_literal_contexts: &mut [usize],
                    mut literal_context_map: &mut [&[u32]]) {
  static mut kStaticContextMapContinuation: [u32; 64] =
    [1u32, 1u32, 2u32, 2u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
     0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
     0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
     0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
     0u32, 0u32, 0u32, 0u32];
  static mut kStaticContextMapSimpleUTF8: [u32; 64] =
    [0u32, 0u32, 1u32, 1u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
     0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
     0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
     0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
     0u32, 0u32, 0u32, 0u32];
  let mut monogram_histo: [u32; 3] = [0u32, 0u32, 0u32];
  let mut two_prefix_histo: [u32; 6] = [0u32, 0u32, 0u32, 0u32, 0u32, 0u32];
  let mut total: usize;
  let mut i: usize;
  let mut dummy: usize;
  let mut entropy: [f64; 4];
  i = 0usize;
  while i < 9usize {
    {
      {
        let _rhs = bigram_histo[(i as (usize))];
        let _lhs = &mut monogram_histo[i.wrapping_rem(3usize)];
        *_lhs = (*_lhs).wrapping_add(_rhs);
      }
      {
        let _rhs = bigram_histo[(i as (usize))];
        let _lhs = &mut two_prefix_histo[i.wrapping_rem(6usize)];
        *_lhs = (*_lhs).wrapping_add(_rhs);
      }
    }
    i = i.wrapping_add(1 as (usize));
  }
  entropy[1usize] = ShannonEntropy(monogram_histo.as_mut_ptr(), 3usize, &mut dummy);
  entropy[2usize] = ShannonEntropy(two_prefix_histo.as_mut_ptr(), 3usize, &mut dummy) +
                    ShannonEntropy(two_prefix_histo.as_mut_ptr().offset(3i32 as (isize)),
                                   3usize,
                                   &mut dummy);
  entropy[3usize] = 0i32 as (f64);
  i = 0usize;
  while i < 3usize {
    {
      let _rhs = ShannonEntropy(bigram_histo[((3usize).wrapping_mul(i) as (usize))..],
                                3usize,
                                &mut dummy);
      let _lhs = &mut entropy[3usize];
      *_lhs = *_lhs + _rhs;
    }
    i = i.wrapping_add(1 as (usize));
  }
  total = monogram_histo[0usize]
    .wrapping_add(monogram_histo[1usize])
    .wrapping_add(monogram_histo[2usize]) as (usize);
  0i32;
  entropy[0usize] = 1.0f64 / total as (f64);
  {
    let _rhs = entropy[0usize];
    let _lhs = &mut entropy[1usize];
    *_lhs = *_lhs * _rhs;
  }
  {
    let _rhs = entropy[0usize];
    let _lhs = &mut entropy[2usize];
    *_lhs = *_lhs * _rhs;
  }
  {
    let _rhs = entropy[0usize];
    let _lhs = &mut entropy[3usize];
    *_lhs = *_lhs * _rhs;
  }
  if quality < 7i32 {
    entropy[3usize] = entropy[1usize] * 10i32 as (f64);
  }
  if entropy[1usize] - entropy[2usize] < 0.2f64 && (entropy[1usize] - entropy[3usize] < 0.2f64) {
    *num_literal_contexts = 1usize;
  } else if entropy[2usize] - entropy[3usize] < 0.02f64 {
    *num_literal_contexts = 2usize;
    *literal_context_map = kStaticContextMapSimpleUTF8.as_ptr();
  } else {
    *num_literal_contexts = 3usize;
    *literal_context_map = kStaticContextMapContinuation.as_ptr();
  }
}

fn DecideOverLiteralContextModeling(mut input: &[u8],
                                    mut start_pos: usize,
                                    mut length: usize,
                                    mut mask: usize,
                                    mut quality: i32,
                                    mut literal_context_mode: &mut [ContextType],
                                    mut num_literal_contexts: &mut [usize],
                                    mut literal_context_map: &mut [&[u32]]) {
  if quality < 5i32 || length < 64usize {
  } else {
    let end_pos: usize = start_pos.wrapping_add(length);
    let mut bigram_prefix_histo: [u32; 9] = [0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32];
    while start_pos.wrapping_add(64usize) <= end_pos {
      {
        static mut lut: [i32; 4] = [0i32, 0i32, 1i32, 2i32];
        let stride_end_pos: usize = start_pos.wrapping_add(64usize);
        let mut prev: i32 = lut[(input[((start_pos & mask) as (usize))] as (i32) >> 6i32) as
        (usize)] * 3i32;
        let mut pos: usize;
        pos = start_pos.wrapping_add(1usize);
        while pos < stride_end_pos {
          {
            let literal: u8 = input[((pos & mask) as (usize))];
            {
              let _rhs = 1;
              let _lhs = &mut bigram_prefix_histo[(prev + lut[(literal as (i32) >> 6i32) as (usize)]) as
                              (usize)];
              *_lhs = (*_lhs).wrapping_add(_rhs as (u32));
            }
            prev = lut[(literal as (i32) >> 6i32) as (usize)] * 3i32;
          }
          pos = pos.wrapping_add(1 as (usize));
        }
      }
      start_pos = start_pos.wrapping_add(4096usize);
    }
    *literal_context_mode = ContextType::CONTEXT_UTF8;
    ChooseContextMap(quality,
                     &mut bigram_prefix_histo[0usize],
                     num_literal_contexts,
                     literal_context_map);
  }
}

fn WriteMetaBlockInternal(mut m: &mut [MemoryManager],
                          mut data: &[u8],
                          mask: usize,
                          last_flush_pos: usize,
                          bytes: usize,
                          is_last: i32,
                          mut params: &[BrotliEncoderParams],
                          prev_byte: u8,
                          prev_byte2: u8,
                          num_literals: usize,
                          num_commands: usize,
                          mut commands: &mut [Command],
                          mut saved_dist_cache: &[i32],
                          mut dist_cache: &mut [i32],
                          mut storage_ix: &mut [usize],
                          mut storage: &mut [u8]) {
  let wrapped_last_flush_pos: u32 = WrapPosition(last_flush_pos);
  let mut last_byte: u8;
  let mut last_byte_bits: u8;
  let mut num_direct_distance_codes: u32 = 0u32;
  let mut distance_postfix_bits: u32 = 0u32;
  if bytes == 0usize {
    BrotliWriteBits(2usize, 3usize, storage_ix, storage);
    *storage_ix = (*storage_ix).wrapping_add(7u32 as (usize)) & !7u32 as (usize);
    return;
  }
  if ShouldCompress(data,
                    mask,
                    last_flush_pos,
                    bytes,
                    num_literals,
                    num_commands) == 0 {
    memcpy(dist_cache,
           saved_dist_cache,
           (4usize).wrapping_mul(::std::mem::size_of::<i32>()));
    BrotliStoreUncompressedMetaBlock(is_last,
                                     data,
                                     wrapped_last_flush_pos as (usize),
                                     mask,
                                     bytes,
                                     storage_ix,
                                     storage);
    return;
  }
  last_byte = storage[(0usize)];
  last_byte_bits = (*storage_ix & 0xffusize) as (u8);
  if (*params).quality >= 10i32 &&
     ((*params).mode as (i32) == BrotliEncoderMode::BROTLI_MODE_FONT as (i32)) {
    num_direct_distance_codes = 12u32;
    distance_postfix_bits = 1u32;
    RecomputeDistancePrefixes(commands,
                              num_commands,
                              num_direct_distance_codes,
                              distance_postfix_bits);
  }
  if (*params).quality <= 2i32 {
    BrotliStoreMetaBlockFast(m,
                             data,
                             wrapped_last_flush_pos as (usize),
                             bytes,
                             mask,
                             is_last,
                             commands,
                             num_commands,
                             storage_ix,
                             storage);
    if !(0i32 == 0) {
      return;
    }
  } else if (*params).quality < 4i32 {
    BrotliStoreMetaBlockTrivial(m,
                                data,
                                wrapped_last_flush_pos as (usize),
                                bytes,
                                mask,
                                is_last,
                                commands,
                                num_commands,
                                storage_ix,
                                storage);
    if !(0i32 == 0) {
      return;
    }
  } else {
    let mut literal_context_mode: ContextType = ContextType::CONTEXT_UTF8;
    let mut mb: MetaBlockSplit;
    InitMetaBlockSplit(&mut mb);
    if (*params).quality < 10i32 {
      let mut num_literal_contexts: usize = 1usize;
      let mut literal_context_map: *const u32 = 0i32;
      if (*params).disable_literal_context_modeling == 0 {
        DecideOverLiteralContextModeling(data,
                                         wrapped_last_flush_pos as (usize),
                                         bytes,
                                         mask,
                                         (*params).quality,
                                         &mut literal_context_mode,
                                         &mut num_literal_contexts,
                                         &mut literal_context_map);
      }
      BrotliBuildMetaBlockGreedy(m,
                                 data,
                                 wrapped_last_flush_pos as (usize),
                                 mask,
                                 prev_byte,
                                 prev_byte2,
                                 literal_context_mode,
                                 num_literal_contexts,
                                 literal_context_map,
                                 commands,
                                 num_commands,
                                 &mut mb);
      if !(0i32 == 0) {
        return;
      }
    } else {
      if BrotliIsMostlyUTF8(data,
                            wrapped_last_flush_pos as (usize),
                            mask,
                            bytes,
                            kMinUTF8Ratio) == 0 {
        literal_context_mode = ContextType::CONTEXT_SIGNED;
      }
      BrotliBuildMetaBlock(m,
                           data,
                           wrapped_last_flush_pos as (usize),
                           mask,
                           params,
                           prev_byte,
                           prev_byte2,
                           commands,
                           num_commands,
                           literal_context_mode,
                           &mut mb);
      if !(0i32 == 0) {
        return;
      }
    }
    if (*params).quality >= 4i32 {
      BrotliOptimizeHistograms(num_direct_distance_codes as (usize),
                               distance_postfix_bits as (usize),
                               &mut mb);
    }
    BrotliStoreMetaBlock(m,
                         data,
                         wrapped_last_flush_pos as (usize),
                         bytes,
                         mask,
                         prev_byte,
                         prev_byte2,
                         is_last,
                         num_direct_distance_codes,
                         distance_postfix_bits,
                         literal_context_mode,
                         commands,
                         num_commands,
                         &mut mb,
                         storage_ix,
                         storage);
    if !(0i32 == 0) {
      return;
    }
    DestroyMetaBlockSplit(m, &mut mb);
  }
  if bytes.wrapping_add(4usize) < *storage_ix >> 3i32 {
    memcpy(dist_cache,
           saved_dist_cache,
           (4usize).wrapping_mul(::std::mem::size_of::<i32>()));
    storage[(0usize)] = last_byte;
    *storage_ix = last_byte_bits as (usize);
    BrotliStoreUncompressedMetaBlock(is_last,
                                     data,
                                     wrapped_last_flush_pos as (usize),
                                     mask,
                                     bytes,
                                     storage_ix,
                                     storage);
  }
}

fn EncodeData(mut s: &mut [BrotliEncoderStateStruct],
              is_last: i32,
              force_flush: i32,
              mut out_size: &mut [usize],
              mut output: &mut [*mut u8])
              -> i32 {
  let delta: usize = UnprocessedInputSize(s);
  let bytes: u32 = delta as (u32);
  let wrapped_last_processed_pos: u32 = WrapPosition((*s).last_processed_pos_);
  let mut data: *mut u8;
  let mut mask: u32;
  let mut m: *mut MemoryManager = &mut (*s).memory_manager_;
  let mut dictionary: *const BrotliDictionary = BrotliGetDictionary();
  if EnsureInitialized(s) == 0 {
    return 0i32;
  }
  data = &mut *(*s).ringbuffer_.data_[((*s).ringbuffer_.buffer_index as (usize))..];
  mask = (*s).ringbuffer_.mask_;
  if (*s).is_last_block_emitted_ != 0 {
    return 0i32;
  }
  if is_last != 0 {
    (*s).is_last_block_emitted_ = 1i32;
  }
  if delta > InputBlockSize(s) {
    return 0i32;
  }
  if (*s).params.quality == 1i32 && (*s).command_buf_.is_null() {
    (*s).command_buf_ = if kCompressFragmentTwoPassBlockSize != 0 {
      BrotliAllocate(m,
                     kCompressFragmentTwoPassBlockSize.wrapping_mul(::std::mem::size_of::<u32>()))
    } else {
      0i32
    };
    (*s).literal_buf_ = if kCompressFragmentTwoPassBlockSize != 0 {
      BrotliAllocate(m,
                     kCompressFragmentTwoPassBlockSize.wrapping_mul(::std::mem::size_of::<u8>()))
    } else {
      0i32
    };
    if !(0i32 == 0) {
      return 0i32;
    }
  }
  if (*s).params.quality == 0i32 || (*s).params.quality == 1i32 {
    let mut storage: *mut u8;
    let mut storage_ix: usize = (*s).last_byte_bits_ as (usize);
    let mut table_size: usize;
    let mut table: *mut i32;
    if delta == 0usize && (is_last == 0) {
      *out_size = 0usize;
      return 1i32;
    }
    storage = GetBrotliStorage(s,
                               (2u32).wrapping_mul(bytes).wrapping_add(502u32) as (usize));
    if !(0i32 == 0) {
      return 0i32;
    }
    storage[(0usize)] = (*s).last_byte_;
    table = GetHashTable(s, (*s).params.quality, bytes as (usize), &mut table_size);
    if !(0i32 == 0) {
      return 0i32;
    }
    if (*s).params.quality == 0i32 {
      BrotliCompressFragmentFast(m,
                                 &mut data[((wrapped_last_processed_pos & mask) as (usize))],
                                 bytes as (usize),
                                 is_last,
                                 table,
                                 table_size,
                                 (*s).cmd_depths_.as_mut_ptr(),
                                 (*s).cmd_bits_.as_mut_ptr(),
                                 &mut (*s).cmd_code_numbits_,
                                 (*s).cmd_code_.as_mut_ptr(),
                                 &mut storage_ix,
                                 storage);
      if !(0i32 == 0) {
        return 0i32;
      }
    } else {
      BrotliCompressFragmentTwoPass(m,
                                    &mut data[((wrapped_last_processed_pos & mask) as (usize))],
                                    bytes as (usize),
                                    is_last,
                                    (*s).command_buf_,
                                    (*s).literal_buf_,
                                    table,
                                    table_size,
                                    &mut storage_ix,
                                    storage);
      if !(0i32 == 0) {
        return 0i32;
      }
    }
    (*s).last_byte_ = storage[((storage_ix >> 3i32) as (usize))];
    (*s).last_byte_bits_ = (storage_ix & 7u32 as (usize)) as (u8);
    UpdateLastProcessedPos(s);
    *output = &mut storage[(0usize)];
    *out_size = storage_ix >> 3i32;
    return 1i32;
  }
  {
    let mut newsize: usize =
      (*s).num_commands_.wrapping_add(bytes.wrapping_div(2u32) as (usize)).wrapping_add(1usize);
    if newsize > (*s).cmd_alloc_size_ {
      let mut new_commands: *mut Command;
      newsize = newsize.wrapping_add(bytes.wrapping_div(4u32).wrapping_add(16u32) as (usize));
      (*s).cmd_alloc_size_ = newsize;
      new_commands = if newsize != 0 {
        BrotliAllocate(m, newsize.wrapping_mul(::std::mem::size_of::<Command>()))
      } else {
        0i32
      };
      if !(0i32 == 0) {
        return 0i32;
      }
      if !(*s).commands_.is_null() {
        memcpy(new_commands,
               (*s).commands_,
               ::std::mem::size_of::<Command>().wrapping_mul((*s).num_commands_));
        {
          BrotliFree(m, (*s).commands_);
          (*s).commands_ = 0i32;
        }
      }
      (*s).commands_ = new_commands;
    }
  }
  InitOrStitchToPreviousBlock(m,
                              &mut (*s).hasher_,
                              data,
                              mask as (usize),
                              &mut (*s).params,
                              wrapped_last_processed_pos as (usize),
                              bytes as (usize),
                              is_last);
  if !(0i32 == 0) {
    return 0i32;
  }
  if (*s).params.quality == 10i32 {
    0i32;
    BrotliCreateZopfliBackwardReferences(m,
                                         dictionary,
                                         bytes as (usize),
                                         wrapped_last_processed_pos as (usize),
                                         data,
                                         mask as (usize),
                                         &mut (*s).params,
                                         (*s).hasher_,
                                         (*s).dist_cache_.as_mut_ptr(),
                                         &mut (*s).last_insert_len_,
                                         &mut *(*s).commands_[((*s).num_commands_ as (usize))..],
                                         &mut (*s).num_commands_,
                                         &mut (*s).num_literals_);
    if !(0i32 == 0) {
      return 0i32;
    }
  } else if (*s).params.quality == 11i32 {
    0i32;
    BrotliCreateHqZopfliBackwardReferences(m,
                                           dictionary,
                                           bytes as (usize),
                                           wrapped_last_processed_pos as (usize),
                                           data,
                                           mask as (usize),
                                           &mut (*s).params,
                                           (*s).hasher_,
                                           (*s).dist_cache_.as_mut_ptr(),
                                           &mut (*s).last_insert_len_,
                                           &mut *(*s).commands_[((*s).num_commands_ as (usize))..],
                                           &mut (*s).num_commands_,
                                           &mut (*s).num_literals_);
    if !(0i32 == 0) {
      return 0i32;
    }
  } else {
    BrotliCreateBackwardReferences(dictionary,
                                   bytes as (usize),
                                   wrapped_last_processed_pos as (usize),
                                   data,
                                   mask as (usize),
                                   &mut (*s).params,
                                   (*s).hasher_,
                                   (*s).dist_cache_.as_mut_ptr(),
                                   &mut (*s).last_insert_len_,
                                   &mut *(*s).commands_[((*s).num_commands_ as (usize))..],
                                   &mut (*s).num_commands_,
                                   &mut (*s).num_literals_);
  }
  {
    let max_length: usize = MaxMetablockSize(&mut (*s).params);
    let max_literals: usize = max_length.wrapping_div(8usize);
    let max_commands: usize = max_length.wrapping_div(8usize);
    let processed_bytes: usize = (*s).input_pos_.wrapping_sub((*s).last_flush_pos_);
    let next_input_fits_metablock: i32 = if !!(processed_bytes.wrapping_add(InputBlockSize(s)) <=
                                               max_length) {
      1i32
    } else {
      0i32
    };
    let should_flush: i32 = if !!((*s).params.quality < 4i32 &&
                                  ((*s).num_literals_.wrapping_add((*s).num_commands_) >=
                                   0x2fffusize)) {
      1i32
    } else {
      0i32
    };
    if is_last == 0 && (force_flush == 0) && (should_flush == 0) &&
       (next_input_fits_metablock != 0) && ((*s).num_literals_ < max_literals) &&
       ((*s).num_commands_ < max_commands) {
      if UpdateLastProcessedPos(s) != 0 {
        HasherReset((*s).hasher_);
      }
      *out_size = 0usize;
      return 1i32;
    }
  }
  if (*s).last_insert_len_ > 0usize {
    InitInsertCommand(&mut *(*s).commands_[({
                               let _old = (*s).num_commands_;
                               (*s).num_commands_ = (*s).num_commands_.wrapping_add(1 as (usize));
                               _old
                             } as (usize))..],
                      (*s).last_insert_len_);
    (*s).num_literals_ = (*s).num_literals_.wrapping_add((*s).last_insert_len_);
    (*s).last_insert_len_ = 0usize;
  }
  if is_last == 0 && ((*s).input_pos_ == (*s).last_flush_pos_) {
    *out_size = 0usize;
    return 1i32;
  }
  0i32;
  0i32;
  0i32;
  {
    let metablock_size: u32 = (*s).input_pos_.wrapping_sub((*s).last_flush_pos_) as (u32);
    let mut storage: *mut u8 =
      GetBrotliStorage(s,
                       (2u32).wrapping_mul(metablock_size).wrapping_add(502u32) as (usize));
    let mut storage_ix: usize = (*s).last_byte_bits_ as (usize);
    if !(0i32 == 0) {
      return 0i32;
    }
    storage[(0usize)] = (*s).last_byte_;
    WriteMetaBlockInternal(m,
                           data,
                           mask as (usize),
                           (*s).last_flush_pos_,
                           metablock_size as (usize),
                           is_last,
                           &mut (*s).params,
                           (*s).prev_byte_,
                           (*s).prev_byte2_,
                           (*s).num_literals_,
                           (*s).num_commands_,
                           (*s).commands_,
                           (*s).saved_dist_cache_.as_mut_ptr(),
                           (*s).dist_cache_.as_mut_ptr(),
                           &mut storage_ix,
                           storage);
    if !(0i32 == 0) {
      return 0i32;
    }
    (*s).last_byte_ = storage[((storage_ix >> 3i32) as (usize))];
    (*s).last_byte_bits_ = (storage_ix & 7u32 as (usize)) as (u8);
    (*s).last_flush_pos_ = (*s).input_pos_;
    if UpdateLastProcessedPos(s) != 0 {
      HasherReset((*s).hasher_);
    }
    if (*s).last_flush_pos_ > 0usize {
      (*s).prev_byte_ = data[((((*s).last_flush_pos_ as (u32)).wrapping_sub(1u32) & mask) as
       (usize))];
    }
    if (*s).last_flush_pos_ > 1usize {
      (*s).prev_byte2_ = data[(((*s).last_flush_pos_.wrapping_sub(2usize) as (u32) & mask) as
       (usize))];
    }
    (*s).num_commands_ = 0usize;
    (*s).num_literals_ = 0usize;
    memcpy((*s).saved_dist_cache_.as_mut_ptr(),
           (*s).dist_cache_.as_mut_ptr(),
           ::std::mem::size_of::<[i32; 4]>());
    *output = &mut storage[(0usize)];
    *out_size = storage_ix >> 3i32;
    1i32
  }
}

fn WriteMetadataHeader(mut s: &mut [BrotliEncoderStateStruct],
                       block_size: usize,
                       mut header: &mut [u8])
                       -> usize {
  let mut storage_ix: usize;
  storage_ix = (*s).last_byte_bits_ as (usize);
  header[(0usize)] = (*s).last_byte_;
  (*s).last_byte_ = 0i32 as (u8);
  (*s).last_byte_bits_ = 0i32 as (u8);
  BrotliWriteBits(1usize, 0usize, &mut storage_ix, header);
  BrotliWriteBits(2usize, 3usize, &mut storage_ix, header);
  BrotliWriteBits(1usize, 0usize, &mut storage_ix, header);
  if block_size == 0usize {
    BrotliWriteBits(2usize, 0usize, &mut storage_ix, header);
  } else {
    let mut nbits: u32 = if block_size == 1usize {
      0u32
    } else {
      Log2FloorNonZero((block_size as (u32)).wrapping_sub(1u32) as (usize)).wrapping_add(1u32)
    };
    let mut nbytes: u32 = nbits.wrapping_add(7u32).wrapping_div(8u32);
    BrotliWriteBits(2usize, nbytes as (usize), &mut storage_ix, header);
    BrotliWriteBits((8u32).wrapping_mul(nbytes) as (usize),
                    block_size.wrapping_sub(1usize),
                    &mut storage_ix,
                    header);
  }
  storage_ix.wrapping_add(7u32 as (usize)) >> 3i32
}

fn brotli_min_uint32_t(mut a: u32, mut b: u32) -> u32 {
  if a < b { a } else { b }
}

fn ProcessMetadata(mut s: &mut [BrotliEncoderStateStruct],
                   mut available_in: &mut [usize],
                   mut next_in: &mut [&[u8]],
                   mut available_out: &mut [usize],
                   mut next_out: &mut [*mut u8],
                   mut total_out: &mut usize)
                   -> i32 {
  if *available_in > (1u32 << 24i32) as (usize) {
    return 0i32;
  }
  if (*s).stream_state_ as (i32) == BrotliEncoderStreamState::BROTLI_STREAM_PROCESSING as (i32) {
    (*s).remaining_metadata_bytes_ = *available_in as (u32);
    (*s).stream_state_ = BrotliEncoderStreamState::BROTLI_STREAM_METADATA_HEAD;
  }
  if (*s).stream_state_ as (i32) !=
     BrotliEncoderStreamState::BROTLI_STREAM_METADATA_HEAD as (i32) &&
     ((*s).stream_state_ as (i32) !=
      BrotliEncoderStreamState::BROTLI_STREAM_METADATA_BODY as (i32)) {
    return 0i32;
  }
  while 1i32 != 0 {
    if InjectFlushOrPushOutput(s, available_out, next_out, total_out) != 0 {
      {
        continue;
      }
    }
    if (*s).available_out_ != 0usize {
      {
        break;
      }
    }
    if (*s).input_pos_ != (*s).last_flush_pos_ {
      let mut result: i32 =
        EncodeData(s, 0i32, 1i32, &mut (*s).available_out_, &mut (*s).next_out_);
      if result == 0 {
        return 0i32;
      }
      {
        {
          continue;
        }
      }
    }
    if (*s).stream_state_ as (i32) ==
       BrotliEncoderStreamState::BROTLI_STREAM_METADATA_HEAD as (i32) {
      (*s).next_out_ = (*s).tiny_buf_.u8.as_mut_ptr();
      (*s).available_out_ =
        WriteMetadataHeader(s, (*s).remaining_metadata_bytes_ as (usize), (*s).next_out_);
      (*s).stream_state_ = BrotliEncoderStreamState::BROTLI_STREAM_METADATA_BODY;
      {
        {
          continue;
        }
      }
    } else {
      if (*s).remaining_metadata_bytes_ == 0u32 {
        (*s).remaining_metadata_bytes_ = !(0u32);
        (*s).stream_state_ = BrotliEncoderStreamState::BROTLI_STREAM_PROCESSING;
        {
          {
            break;
          }
        }
      }
      if *available_out != 0 {
        let mut copy: u32 = brotli_min_size_t((*s).remaining_metadata_bytes_ as (usize),
                                              *available_out) as (u32);
        memcpy(*next_out, *next_in, copy as (usize));
        *next_in = (*next_in).offset(copy as (isize));
        *available_in = (*available_in).wrapping_sub(copy as (usize));
        (*s).remaining_metadata_bytes_ = (*s).remaining_metadata_bytes_.wrapping_sub(copy);
        *next_out = (*next_out).offset(copy as (isize));
        *available_out = (*available_out).wrapping_sub(copy as (usize));
      } else {
        let mut copy: u32 = brotli_min_uint32_t((*s).remaining_metadata_bytes_, 16u32);
        (*s).next_out_ = (*s).tiny_buf_.u8.as_mut_ptr();
        memcpy((*s).next_out_, *next_in, copy as (usize));
        *next_in = (*next_in).offset(copy as (isize));
        *available_in = (*available_in).wrapping_sub(copy as (usize));
        (*s).remaining_metadata_bytes_ = (*s).remaining_metadata_bytes_.wrapping_sub(copy);
        (*s).available_out_ = copy as (usize);
      }
      {
        {
          continue;
        }
      }
    }
  }
  1i32
}

fn CheckFlushComplete(mut s: &mut [BrotliEncoderStateStruct]) {
  if (*s).stream_state_ as (i32) ==
     BrotliEncoderStreamState::BROTLI_STREAM_FLUSH_REQUESTED as (i32) &&
     ((*s).available_out_ == 0usize) {
    (*s).stream_state_ = BrotliEncoderStreamState::BROTLI_STREAM_PROCESSING;
    (*s).next_out_ = 0i32;
  }
}

fn BrotliEncoderCompressStreamFast(mut s: &mut [BrotliEncoderStateStruct],
                                   mut op: BrotliEncoderOperation,
                                   mut available_in: &mut [usize],
                                   mut next_in: &mut [&[u8]],
                                   mut available_out: &mut [usize],
                                   mut next_out: &mut [*mut u8],
                                   mut total_out: &mut usize)
                                   -> i32 {
  let block_size_limit: usize = 1usize << (*s).params.lgwin;
  let buf_size: usize = brotli_min_size_t(kCompressFragmentTwoPassBlockSize,
                                          brotli_min_size_t(*available_in, block_size_limit));
  let mut tmp_command_buf: *mut u32 = 0i32;
  let mut command_buf: *mut u32 = 0i32;
  let mut tmp_literal_buf: *mut u8 = 0i32;
  let mut literal_buf: *mut u8 = 0i32;
  let mut m: *mut MemoryManager = &mut (*s).memory_manager_;
  if (*s).params.quality != 0i32 && ((*s).params.quality != 1i32) {
    return 0i32;
  }
  if (*s).params.quality == 1i32 {
    if (*s).command_buf_.is_null() && (buf_size == kCompressFragmentTwoPassBlockSize) {
      (*s).command_buf_ = if kCompressFragmentTwoPassBlockSize != 0 {
        BrotliAllocate(m,
                       kCompressFragmentTwoPassBlockSize.wrapping_mul(::std::mem::size_of::<u32>()))
      } else {
        0i32
      };
      (*s).literal_buf_ = if kCompressFragmentTwoPassBlockSize != 0 {
        BrotliAllocate(m,
                       kCompressFragmentTwoPassBlockSize.wrapping_mul(::std::mem::size_of::<u8>()))
      } else {
        0i32
      };
      if !(0i32 == 0) {
        return 0i32;
      }
    }
    if !(*s).command_buf_.is_null() {
      command_buf = (*s).command_buf_;
      literal_buf = (*s).literal_buf_;
    } else {
      tmp_command_buf = if buf_size != 0 {
        BrotliAllocate(m, buf_size.wrapping_mul(::std::mem::size_of::<u32>()))
      } else {
        0i32
      };
      tmp_literal_buf = if buf_size != 0 {
        BrotliAllocate(m, buf_size.wrapping_mul(::std::mem::size_of::<u8>()))
      } else {
        0i32
      };
      if !(0i32 == 0) {
        return 0i32;
      }
      command_buf = tmp_command_buf;
      literal_buf = tmp_literal_buf;
    }
  }
  while 1i32 != 0 {
    if InjectFlushOrPushOutput(s, available_out, next_out, total_out) != 0 {
      {
        continue;
      }
    }
    if (*s).available_out_ == 0usize &&
       ((*s).stream_state_ as (i32) ==
        BrotliEncoderStreamState::BROTLI_STREAM_PROCESSING as (i32)) &&
       (*available_in != 0usize ||
        op as (i32) != BrotliEncoderOperation::BROTLI_OPERATION_PROCESS as (i32)) {
      let mut block_size: usize = brotli_min_size_t(block_size_limit, *available_in);
      let mut is_last: i32 = (*available_in == block_size &&
                              (op as (i32) ==
                               BrotliEncoderOperation::BROTLI_OPERATION_FINISH as (i32))) as
                             (i32);
      let mut force_flush: i32 =
        (*available_in == block_size &&
         (op as (i32) == BrotliEncoderOperation::BROTLI_OPERATION_FLUSH as (i32))) as (i32);
      let mut max_out_size: usize = (2usize).wrapping_mul(block_size).wrapping_add(502usize);
      let mut inplace: i32 = 1i32;
      let mut storage: *mut u8 = 0i32;
      let mut storage_ix: usize = (*s).last_byte_bits_ as (usize);
      let mut table_size: usize;
      let mut table: *mut i32;
      if force_flush != 0 && (block_size == 0usize) {
        (*s).stream_state_ = BrotliEncoderStreamState::BROTLI_STREAM_FLUSH_REQUESTED;
        {
          {
            continue;
          }
        }
      }
      if max_out_size <= *available_out {
        storage = *next_out;
      } else {
        inplace = 0i32;
        storage = GetBrotliStorage(s, max_out_size);
        if !(0i32 == 0) {
          return 0i32;
        }
      }
      storage[(0usize)] = (*s).last_byte_;
      table = GetHashTable(s, (*s).params.quality, block_size, &mut table_size);
      if !(0i32 == 0) {
        return 0i32;
      }
      if (*s).params.quality == 0i32 {
        BrotliCompressFragmentFast(m,
                                   *next_in,
                                   block_size,
                                   is_last,
                                   table,
                                   table_size,
                                   (*s).cmd_depths_.as_mut_ptr(),
                                   (*s).cmd_bits_.as_mut_ptr(),
                                   &mut (*s).cmd_code_numbits_,
                                   (*s).cmd_code_.as_mut_ptr(),
                                   &mut storage_ix,
                                   storage);
        if !(0i32 == 0) {
          return 0i32;
        }
      } else {
        BrotliCompressFragmentTwoPass(m,
                                      *next_in,
                                      block_size,
                                      is_last,
                                      command_buf,
                                      literal_buf,
                                      table,
                                      table_size,
                                      &mut storage_ix,
                                      storage);
        if !(0i32 == 0) {
          return 0i32;
        }
      }
      *next_in = (*next_in).offset(block_size as (isize));
      *available_in = (*available_in).wrapping_sub(block_size);
      if inplace != 0 {
        let mut out_bytes: usize = storage_ix >> 3i32;
        0i32;
        0i32;
        *next_out = (*next_out).offset(out_bytes as (isize));
        *available_out = (*available_out).wrapping_sub(out_bytes);
        (*s).total_out_ = (*s).total_out_.wrapping_add(out_bytes);
        if !total_out.is_null() {
          *total_out = (*s).total_out_;
        }
      } else {
        let mut out_bytes: usize = storage_ix >> 3i32;
        (*s).next_out_ = storage;
        (*s).available_out_ = out_bytes;
      }
      (*s).last_byte_ = storage[((storage_ix >> 3i32) as (usize))];
      (*s).last_byte_bits_ = (storage_ix & 7u32 as (usize)) as (u8);
      if force_flush != 0 {
        (*s).stream_state_ = BrotliEncoderStreamState::BROTLI_STREAM_FLUSH_REQUESTED;
      }
      if is_last != 0 {
        (*s).stream_state_ = BrotliEncoderStreamState::BROTLI_STREAM_FINISHED;
      }
      {
        {
          continue;
        }
      }
    }
    {
      {
        break;
      }
    }
  }
  {
    BrotliFree(m, tmp_command_buf);
    tmp_command_buf = 0i32;
  }
  {
    BrotliFree(m, tmp_literal_buf);
    tmp_literal_buf = 0i32;
  }
  CheckFlushComplete(s);
  1i32
}

fn RemainingInputBlockSize(mut s: &mut [BrotliEncoderStateStruct]) -> usize {
  let delta: usize = UnprocessedInputSize(s);
  let mut block_size: usize = InputBlockSize(s);
  if delta >= block_size {
    return 0usize;
  }
  block_size.wrapping_sub(delta)
}


pub fn BrotliEncoderCompressStream(mut s: &mut [BrotliEncoderStateStruct],
                                   mut op: BrotliEncoderOperation,
                                   mut available_in: &mut [usize],
                                   mut next_in: &mut [&[u8]],
                                   mut available_out: &mut [usize],
                                   mut next_out: &mut [*mut u8],
                                   mut total_out: &mut usize)
                                   -> i32 {
  if EnsureInitialized(s) == 0 {
    return 0i32;
  }
  if (*s).remaining_metadata_bytes_ != !(0u32) {
    if *available_in != (*s).remaining_metadata_bytes_ as (usize) {
      return 0i32;
    }
    if op as (i32) != BrotliEncoderOperation::BROTLI_OPERATION_EMIT_METADATA as (i32) {
      return 0i32;
    }
  }
  if op as (i32) == BrotliEncoderOperation::BROTLI_OPERATION_EMIT_METADATA as (i32) {
    UpdateSizeHint(s, 0usize);
    return ProcessMetadata(s, available_in, next_in, available_out, next_out, total_out);
  }
  if (*s).stream_state_ as (i32) ==
     BrotliEncoderStreamState::BROTLI_STREAM_METADATA_HEAD as (i32) ||
     (*s).stream_state_ as (i32) == BrotliEncoderStreamState::BROTLI_STREAM_METADATA_BODY as (i32) {
    return 0i32;
  }
  if (*s).stream_state_ as (i32) !=
     BrotliEncoderStreamState::BROTLI_STREAM_PROCESSING as (i32) && (*available_in != 0usize) {
    return 0i32;
  }
  if (*s).params.quality == 0i32 || (*s).params.quality == 1i32 {
    return BrotliEncoderCompressStreamFast(s,
                                           op,
                                           available_in,
                                           next_in,
                                           available_out,
                                           next_out,
                                           total_out);
  }
  while 1i32 != 0 {
    let mut remaining_block_size: usize = RemainingInputBlockSize(s);
    if remaining_block_size != 0usize && (*available_in != 0usize) {
      let mut copy_input_size: usize = brotli_min_size_t(remaining_block_size, *available_in);
      CopyInputToRingBuffer(s, copy_input_size, *next_in);
      *next_in = (*next_in).offset(copy_input_size as (isize));
      *available_in = (*available_in).wrapping_sub(copy_input_size);
      {
        {
          continue;
        }
      }
    }
    if InjectFlushOrPushOutput(s, available_out, next_out, total_out) != 0 {
      {
        continue;
      }
    }
    if (*s).available_out_ == 0usize &&
       ((*s).stream_state_ as (i32) ==
        BrotliEncoderStreamState::BROTLI_STREAM_PROCESSING as (i32)) {
      if remaining_block_size == 0usize ||
         op as (i32) != BrotliEncoderOperation::BROTLI_OPERATION_PROCESS as (i32) {
        let mut is_last: i32 = if !!(*available_in == 0usize &&
                                     (op as (i32) ==
                                      BrotliEncoderOperation::BROTLI_OPERATION_FINISH as (i32))) {
          1i32
        } else {
          0i32
        };
        let mut force_flush: i32 =
          if !!(*available_in == 0usize &&
                (op as (i32) == BrotliEncoderOperation::BROTLI_OPERATION_FLUSH as (i32))) {
            1i32
          } else {
            0i32
          };
        let mut result: i32;
        UpdateSizeHint(s, *available_in);
        result = EncodeData(s,
                            is_last,
                            force_flush,
                            &mut (*s).available_out_,
                            &mut (*s).next_out_);
        if result == 0 {
          return 0i32;
        }
        if force_flush != 0 {
          (*s).stream_state_ = BrotliEncoderStreamState::BROTLI_STREAM_FLUSH_REQUESTED;
        }
        if is_last != 0 {
          (*s).stream_state_ = BrotliEncoderStreamState::BROTLI_STREAM_FINISHED;
        }
        {
          {
            continue;
          }
        }
      }
    }
    {
      {
        break;
      }
    }
  }
  CheckFlushComplete(s);
  1i32
}


pub fn BrotliEncoderIsFinished(mut s: &mut [BrotliEncoderStateStruct]) -> i32 {
  if !!((*s).stream_state_ as (i32) == BrotliEncoderStreamState::BROTLI_STREAM_FINISHED as (i32) &&
        (BrotliEncoderHasMoreOutput(s) == 0)) {
    1i32
  } else {
    0i32
  }
}


pub fn BrotliEncoderHasMoreOutput(mut s: &mut [BrotliEncoderStateStruct]) -> i32 {
  if !!((*s).available_out_ != 0usize) {
    1i32
  } else {
    0i32
  }
}


pub fn BrotliEncoderTakeOutput(mut s: &mut [BrotliEncoderStateStruct],
                               mut size: &mut [usize])
                               -> *const u8 {
  let mut consumed_size: usize = (*s).available_out_;
  let mut result: *mut u8 = (*s).next_out_;
  if *size != 0 {
    consumed_size = brotli_min_size_t(*size, (*s).available_out_);
  }
  if consumed_size != 0 {
    (*s).next_out_ = (*s).next_out_[(consumed_size as (usize))..];
    (*s).available_out_ = (*s).available_out_.wrapping_sub(consumed_size);
    (*s).total_out_ = (*s).total_out_.wrapping_add(consumed_size);
    CheckFlushComplete(s);
    *size = consumed_size;
  } else {
    *size = 0usize;
    result = 0i32;
  }
  result
}


pub fn BrotliEncoderVersion() -> u32 {
  0x1000000u32
}


pub fn BrotliEncoderInputBlockSize(mut s: &mut [BrotliEncoderStateStruct]) -> usize {
  InputBlockSize(s)
}


pub fn BrotliEncoderCopyInputToRingBuffer(mut s: &mut [BrotliEncoderStateStruct],
                                          input_size: usize,
                                          mut input_buffer: &[u8]) {
  CopyInputToRingBuffer(s, input_size, input_buffer);
}


pub fn BrotliEncoderWriteData(mut s: &mut [BrotliEncoderStateStruct],
                              is_last: i32,
                              force_flush: i32,
                              mut out_size: &mut [usize],
                              mut output: &mut [*mut u8])
                              -> i32 {
  EncodeData(s, is_last, force_flush, out_size, output)
}
 */