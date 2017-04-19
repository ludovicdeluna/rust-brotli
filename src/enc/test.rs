#![cfg(test)]
use core;
extern crate alloc_no_stdlib;
use super::super::alloc::{AllocatedStackMemory, Allocator, SliceWrapper, SliceWrapperMut, StackAllocator, bzero};
use super::encode::{BrotliEncoderCreateInstance};
use super::histogram::{ContextType, HistogramLiteral, HistogramCommand, HistogramDistance};
use super::cluster::{HistogramPair};
#[cfg(not(feature="no-stdlib"))]
use std::vec::Vec;
#[cfg(not(feature="no-stdlib"))]
use std::io;
extern {
  fn calloc(n_elem : usize, el_size : usize) -> *mut u8;
}
extern {
  fn free(ptr : *mut u8);
}

use core::ops;
use super::entropy_encode::{HuffmanTree};
use super::command::{Command};
pub use super::super::{BrotliDecompressStream, BrotliResult, BrotliState, HuffmanCode};

declare_stack_allocator_struct!(MemPool, 4096, stack);
declare_stack_allocator_struct!(CallocatedFreelist4096, 4096, calloc);
declare_stack_allocator_struct!(CallocatedFreelist2048, 2048, calloc);

fn oneshot_compress(input: &mut [u8], mut output: &mut [u8]) -> (i32, usize) {
  let mut available_out: usize = output.len();
  let mut stack_u8_buffer = unsafe{define_allocator_memory_pool!(4096, u8, [0; 1024 * 1024], calloc)};
  let mut stack_u16_buffer = unsafe{define_allocator_memory_pool!(4096, u16, [0; 128 * 1024], calloc)};
  let mut stack_i32_buffer = unsafe{define_allocator_memory_pool!(4096, i32, [0; 128 * 1024], calloc)};
  let mut stack_u32_buffer = unsafe{define_allocator_memory_pool!(4096, u32, [0; 128 * 1024], calloc)};
  let mut stack_f64_buffer = unsafe{define_allocator_memory_pool!(2048, f64, [0; 128 * 1024], calloc)};
  let mut stack_hl_buffer = unsafe{define_allocator_memory_pool!(2048, HistogramLiteral, [0; 128 * 1024], calloc)};
  let mut stack_hc_buffer = unsafe{define_allocator_memory_pool!(2048, HistogramCommand, [0; 128 * 1024], calloc)};
  let mut stack_hd_buffer = unsafe{define_allocator_memory_pool!(2048, HistogramDistance, [0; 128 * 1024], calloc)};
  let mut stack_hp_buffer = unsafe{define_allocator_memory_pool!(2048, HistogramPair, [0; 128 * 1024], calloc)};
  let mut stack_ct_buffer = unsafe{define_allocator_memory_pool!(2048, ContextType, [0; 128 * 1024], calloc)};
  let mut stack_ht_buffer = unsafe{define_allocator_memory_pool!(2048, HuffmanTree, [0; 128 * 1024], calloc)};
  let mut stack_mc_buffer = unsafe{define_allocator_memory_pool!(2048, Command, [0; 128 * 1024], calloc)};
  let stack_u8_allocator = CallocatedFreelist4096::<u8>::new_allocator(stack_u8_buffer.data, bzero);
  let stack_u16_allocator = CallocatedFreelist4096::<u16>::new_allocator(stack_u16_buffer.data, bzero);
  let stack_i32_allocator = CallocatedFreelist4096::<i32>::new_allocator(stack_i32_buffer.data, bzero);
  let stack_u32_allocator = CallocatedFreelist4096::<u32>::new_allocator(stack_u32_buffer.data, bzero);
  let stack_f64_allocator = CallocatedFreelist2048::<f64>::new_allocator(stack_f64_buffer.data, bzero);
  let stack_mc_allocator = CallocatedFreelist2048::<Command>::new_allocator(stack_mc_buffer.data, bzero);
  let stack_hl_allocator = CallocatedFreelist2048::<HistogramLiteral>::new_allocator(stack_hl_buffer.data, bzero);
  let stack_hc_allocator = CallocatedFreelist2048::<HistogramCommand>::new_allocator(stack_hc_buffer.data, bzero);
  let stack_hd_allocator = CallocatedFreelist2048::<HistogramDistance>::new_allocator(stack_hd_buffer.data, bzero);
  let stack_hp_allocator = CallocatedFreelist2048::<HistogramPair>::new_allocator(stack_hp_buffer.data, bzero);
  let stack_ct_allocator = CallocatedFreelist2048::<ContextType>::new_allocator(stack_ct_buffer.data, bzero);
  let stack_ht_allocator = CallocatedFreelist2048::<HuffmanTree>::new_allocator(stack_ht_buffer.data, bzero);
  let mut s_orig = BrotliEncoderCreateInstance(stack_u8_allocator,
                                               stack_u16_allocator,
                                               stack_i32_allocator,
                                               stack_u32_allocator,
                                               stack_mc_allocator);
  return (0,0);
}
 
fn oneshot_decompress(mut compressed: &mut [u8], mut output: &mut [u8]) -> (BrotliResult, usize, usize) {
  let mut available_in: usize = compressed.len();
  let mut available_out: usize = output.len();
  let mut stack_u8_buffer = define_allocator_memory_pool!(4096, u8, [0; 100 * 1024], stack);
  let mut stack_u32_buffer = define_allocator_memory_pool!(4096, u32, [0; 12 * 1024], stack);
  let mut stack_hc_buffer = define_allocator_memory_pool!(4096,
                                                          HuffmanCode,
                                                          [HuffmanCode::default(); 18 * 1024],
                                                          stack);

  let stack_u8_allocator = MemPool::<u8>::new_allocator(&mut stack_u8_buffer, bzero);
  let stack_u32_allocator = MemPool::<u32>::new_allocator(&mut stack_u32_buffer, bzero);
  let stack_hc_allocator = MemPool::<HuffmanCode>::new_allocator(&mut stack_hc_buffer, bzero);
  let mut input_offset: usize = 0;
  let mut output_offset: usize = 0;
  let mut written: usize = 0;
  let mut brotli_state =
    BrotliState::new(stack_u8_allocator, stack_u32_allocator, stack_hc_allocator);
  let result = BrotliDecompressStream(&mut available_in,
                                      &mut input_offset,
                                      &compressed[..],
                                      &mut available_out,
                                      &mut output_offset,
                                      &mut output,
                                      &mut written,
                                      &mut brotli_state);
  brotli_state.BrotliStateCleanup();
  return (result, input_offset, output_offset);

}

fn oneshot(input: &mut [u8], mut compressed: &mut [u8], mut output: &mut [u8]) -> (BrotliResult, usize, usize) {
  let (success, mut available_in) = oneshot_compress(input, compressed);
  if success == 0 {
      //return (BrotliResult::ResultFailure, 0, 0);
      available_in = compressed.len();
  }
  return oneshot_decompress(&mut compressed[..available_in], output);
}

#[test]
fn test_roundtrip_10x10y() {
  const BUFFER_SIZE: usize = 16384;
  let mut compressed: [u8; 12] = [0x1b, 0x13, 0x00, 0x00, 0xa4, 0xb0, 0xb2, 0xea, 0x81, 0x47, 0x02,
                             0x8a];
  let mut output = [0u8; BUFFER_SIZE];
  let mut input  = [0u8; BUFFER_SIZE];
  let (result, compressed_offset, output_offset) = oneshot(&mut input[..], &mut compressed, &mut output[..]);
  match result {
    BrotliResult::ResultSuccess => {}
    _ => assert!(false),
  }
  let mut i: usize = 0;
  while i < 10 {
    assert_eq!(output[i], 'X' as u8);
    assert_eq!(output[i + 10], 'Y' as u8);
    i += 1;
  }
  assert_eq!(output_offset, 20);
  assert_eq!(compressed_offset, compressed.len());
}



#[test]
fn test_roundtrip_x() {
  const BUFFER_SIZE: usize = 16384;
  let mut compressed: [u8; 5] = [0x0b, 0x00, 0x80, 0x58, 0x03];
  let mut output = [0u8; BUFFER_SIZE];
  let mut input = [0u8; BUFFER_SIZE];
  let (result, compressed_offset, output_offset) = oneshot(&mut input[..], &mut compressed[..], &mut output[..]);
  match result {
    BrotliResult::ResultSuccess => {}
    _ => assert!(false),
  }
  assert_eq!(output[0], 'X' as u8);
  assert_eq!(output_offset, 1);
  assert_eq!(compressed_offset, compressed.len());
}

#[test]
fn test_roundtrip_empty() {
  const BUFFER_SIZE: usize = 16384;
  let mut compressed: [u8; 1] = [0x06];
  let mut output = [0u8; 1];
  let (result, compressed_offset, output_offset) = oneshot(&mut [], &mut compressed[..], &mut output[..]);
  match result {
    BrotliResult::ResultSuccess => {}
    _ => assert!(false),
  }
  assert_eq!(output_offset, 0);
  assert_eq!(compressed_offset, compressed.len());
}
const QF_BUFFER_SIZE: usize = 180 * 1024;
static mut quick_fox_output: [u8; QF_BUFFER_SIZE] = [0u8; QF_BUFFER_SIZE];



#[cfg(not(feature="no-stdlib"))]
struct Buffer {
  data: Vec<u8>,
  read_offset: usize,
}
#[cfg(not(feature="no-stdlib"))]
impl Buffer {
  pub fn new(buf: &[u8]) -> Buffer {
    let mut ret = Buffer {
      data: Vec::<u8>::new(),
      read_offset: 0,
    };
    ret.data.extend(buf);
    return ret;
  }
}
#[cfg(not(feature="no-stdlib"))]
impl io::Read for Buffer {
  fn read(self: &mut Self, buf: &mut [u8]) -> io::Result<usize> {
    let bytes_to_read = ::core::cmp::min(buf.len(), self.data.len() - self.read_offset);
    if bytes_to_read > 0 {
      buf[0..bytes_to_read]
        .clone_from_slice(&self.data[self.read_offset..self.read_offset + bytes_to_read]);
    }
    self.read_offset += bytes_to_read;
    return Ok(bytes_to_read);
  }
}
#[cfg(not(feature="no-stdlib"))]
impl io::Write for Buffer {
  fn write(self: &mut Self, buf: &[u8]) -> io::Result<usize> {
    self.data.extend(buf);
    return Ok(buf.len());
  }
  fn flush(self: &mut Self) -> io::Result<()> {
    return Ok(());
  }
}

