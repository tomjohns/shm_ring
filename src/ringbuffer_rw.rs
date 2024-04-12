use core::slice;
use std::{mem::{transmute, size_of}, fmt::{Formatter, Display}};
#[cfg(feature = "avx2")]
use crate::avx::SliceExt;

#[derive(Debug)]
pub struct RingbufRw <'a> {
    head : &'a usize,
    tail : &'a mut usize,
    size : usize,
    buffer : &'a mut [u8],
}

impl <'a> RingbufRw <'a> {
    pub fn make(tail : & 'a mut usize, head : & 'a usize, size : usize, buffer : & 'a mut [u8]) -> Self {
        Self {
            tail : tail,
            head : head,
            size : size,//TODO: get rid of size, we can get it from buffer.len()
            buffer : buffer,
        }
    }
    
    pub fn new(size : usize, data : * mut u8) -> Self {
        if data.is_null() {panic!("data cannot be null")}
        let tail : & mut usize = unsafe { transmute(data as * mut usize) };
        let data = unsafe { data.offset(size_of::<usize>() as isize) };
        let head : & usize = unsafe { transmute(data as * mut usize) };
        let data = unsafe { data.offset(size_of::<usize>() as isize) };
        let size = size - (size_of::<usize>() * 2);
        return RingbufRw::make(tail, head, size, unsafe {slice::from_raw_parts_mut(data, size)} );
    }

    pub fn is_empty(&self) -> bool {
        if *self.head == *self.tail{
            true
        } else {
            false
        } 
    }

    pub fn is_full(&self) -> bool {
        if (*self.tail + 1) % self.size == *self.head {
            true
        } else {
            false
        }
    }

    pub fn get_curr_bytes(&self) -> usize {
        let head = *self.head;
        let tail = *self.tail;
        if tail > head {
            tail - head
        } else if tail < head {
            self.size + tail - head
        } else {
            0
        }
    }
    
    pub fn get_head(&self) -> usize {
        *self.head
    }
    
    pub fn get_tail(&self) -> usize {
        *self.tail
    }

    pub fn get_size(&self) -> usize {
        self.size
    }

    pub fn empty_slots_left(&self) -> usize {
        self.size - self.get_curr_bytes() - 1
    }
    #[cfg(feature = "avx2")]
    pub fn push(&mut self, msg: &[u8]) -> usize {
        //is buffer full?
        //is there room for the message
        let sz_of_usize = size_of::<usize>();
        let msg_len = msg.len();

        if self.is_full() || msg_len + sz_of_usize > self.empty_slots_left() {return 0;}

        let tail = *self.tail;
        let msg_len_bytes = msg_len.to_le_bytes();
        let msg_len_bytes_len = msg_len_bytes.len();
        let bytes_until_end = self.size - tail;

        if bytes_until_end < sz_of_usize {
//EXTRA SAD CASE
            self.buffer[tail..].copy_from_slice_avx(&msg_len_bytes[..bytes_until_end]);
            self.buffer[..(msg_len_bytes_len-bytes_until_end)].copy_from_slice_avx(&msg_len_bytes[bytes_until_end..]);
        
            self.buffer[(msg_len_bytes_len-bytes_until_end)..(msg_len_bytes_len-bytes_until_end)+msg_len].copy_from_slice_avx(msg);
            
            *self.tail = msg_len_bytes_len -bytes_until_end + msg_len;
//SAD CASE
        } else if bytes_until_end <= sz_of_usize + msg_len {
            self.buffer[tail..tail+sz_of_usize].copy_from_slice_avx(&msg_len_bytes);

            self.buffer[tail+sz_of_usize..].copy_from_slice_avx(&msg[..(bytes_until_end-sz_of_usize)]);
            self.buffer[..msg_len+sz_of_usize-bytes_until_end].copy_from_slice_avx(&msg[(bytes_until_end-sz_of_usize)..]);

            *self.tail = msg_len + sz_of_usize - bytes_until_end;
//HAPPY CASE
        } else {
            self.buffer[tail..tail+sz_of_usize].copy_from_slice_avx(&msg_len_bytes);
            self.buffer[tail+sz_of_usize..tail+sz_of_usize+msg_len].copy_from_slice_avx(msg);
            *self.tail = (tail+sz_of_usize+msg_len) % self.size;
        }

        msg_len + sz_of_usize
    }

    #[cfg(not(feature = "avx2"))]
    pub fn push(&mut self, msg: &[u8]) -> usize {
        //is buffer full?
        //is there room for the message
        let sz_of_usize = size_of::<usize>();
        let msg_len = msg.len();

        if self.is_full() || msg_len + sz_of_usize > self.empty_slots_left() {return 0;}

        let tail = *self.tail;
        let msg_len_bytes = msg_len.to_le_bytes();
        let msg_len_bytes_len = msg_len_bytes.len();
        let bytes_until_end = self.size - tail;

        if bytes_until_end < sz_of_usize {
//EXTRA SAD CASE
            self.buffer[tail..].copy_from_slice(&msg_len_bytes[..bytes_until_end]);
            self.buffer[..(msg_len_bytes_len-bytes_until_end)].copy_from_slice(&msg_len_bytes[bytes_until_end..]);
        
            self.buffer[(msg_len_bytes_len-bytes_until_end)..(msg_len_bytes_len-bytes_until_end)+msg_len].copy_from_slice(msg);
            
            *self.tail = msg_len_bytes_len -bytes_until_end + msg_len;
//SAD CASE
        } else if bytes_until_end <= sz_of_usize + msg_len {
            self.buffer[tail..tail+sz_of_usize].copy_from_slice(&msg_len_bytes);

            self.buffer[tail+sz_of_usize..].copy_from_slice(&msg[..(bytes_until_end-sz_of_usize)]);
            self.buffer[..msg_len+sz_of_usize-bytes_until_end].copy_from_slice(&msg[(bytes_until_end-sz_of_usize)..]);

            *self.tail = msg_len + sz_of_usize - bytes_until_end;
//HAPPY CASE
        } else {
            self.buffer[tail..tail+sz_of_usize].copy_from_slice(&msg_len_bytes);
            self.buffer[tail+sz_of_usize..tail+sz_of_usize+msg_len].copy_from_slice(msg);
            *self.tail = (tail+sz_of_usize+msg_len) % self.size;
        }

        msg_len + sz_of_usize
    }
}

impl<'a> Display for RingbufRw<'a> {
    fn fmt(&self, format : &mut Formatter) -> Result<(), std::fmt::Error>{
        let hex: String = self.buffer.iter().map(|&byte| format!("{: >5x}", byte)).collect::<Vec<String>>().join("|");
        let headtail: String = self.buffer.iter().enumerate()
        .map(|(i, &_byte)| {
            if self.is_empty() {
                String::from("EMPTY")
            } else if self.is_full() {
                String::from("FULLL")
            } else if i == *self.head {
                String::from("HEAD^")
            } else if i == *self.tail {
                String::from("TAIL^")
            } else {
                String::from("     ")
            }
        })
        .collect::<Vec<String>>().join("|");
        
        return write!(format, "\nRing Buffer: tail: {}, head: {}, size: {}\n [ {} ]\n [ {} ]\n",
                      self.tail,
                      self.head,
                      self.size,
                      &hex,
                      &headtail);
    }
}


