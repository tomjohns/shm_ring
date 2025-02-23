use core::slice;
use std::{mem::size_of, fmt::{Formatter, Display}};
use crate::SZ_OF_USIZE;
#[cfg(feature = "avx2")]
use crate::avx::SliceExt;

#[derive(Debug)]
pub struct RingbufRw <'a> {
    pub(crate) head : &'a usize,
    pub(crate) tail : &'a mut usize,
    pub(crate) buffer : &'a mut [u8],
}

impl <'a> RingbufRw <'a> {
    pub fn make(tail : & 'a mut usize, head : & 'a usize, buffer : & 'a mut [u8]) -> Self {
        Self { tail, head, buffer }
    }

    /// # Safety
    ///
    /// This function is used to create a ringbuffer from a pointer and length
    /// It is up to the caller to ensure the size argument is correct 
    pub unsafe fn new(size : usize, data : * mut u8) -> Self {
        if data.is_null() {panic!("data cannot be null")}
        let tail : &mut usize = unsafe { &mut *(data as * mut usize) };
        let data = unsafe { data.add(SZ_OF_USIZE) };
        let head : &usize = unsafe { &*(data as * mut usize) };
        let data = unsafe { data.add(SZ_OF_USIZE) };
        let size = size - (size_of::<usize>() * 2);
        return RingbufRw::make(tail, head,unsafe {slice::from_raw_parts_mut(data, size)} );
    }

    pub fn is_empty(&self) -> bool {
        *self.head == *self.tail
    }

    pub fn is_full(&self) -> bool {
        (*self.tail + 1) % self.buffer.len() == *self.head 
    }

    pub fn get_curr_bytes(&self) -> usize {
        let head = *self.head;
        let tail = *self.tail;
        if tail > head {
            tail - head
        } else if tail < head {
            self.buffer.len() + tail - head
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

    pub fn set_tail(&mut self, num: usize) {
        *self.tail = num;
    }

    pub fn get_size(&self) -> usize {
        self.buffer.len()
    }

    pub fn empty_slots_left(&self) -> usize {
        self.buffer.len() - self.get_curr_bytes() - 1
    }
    #[cfg(feature = "avx2")]
    pub fn push(&mut self, msg: &[u8]) -> usize {
        //is buffer full?
        //is there room for the message
        let msg_len = msg.len();

        if self.is_full() || msg_len + SZ_OF_USIZE > self.empty_slots_left() {return 0;}

        let tail = *self.tail;
        let msg_len_bytes = msg_len.to_le_bytes();
        let msg_len_bytes_len = msg_len_bytes.len();
        let bytes_until_end = self.buffer.len() - tail;

        if bytes_until_end < SZ_OF_USIZE {
//EXTRA SAD CASE
            self.buffer[tail..].copy_from_slice_avx(&msg_len_bytes[..bytes_until_end]);
            self.buffer[..(msg_len_bytes_len-bytes_until_end)].copy_from_slice_avx(&msg_len_bytes[bytes_until_end..]);
        
            self.buffer[(msg_len_bytes_len-bytes_until_end)..(msg_len_bytes_len-bytes_until_end)+msg_len].copy_from_slice_avx(msg);
            
            *self.tail = msg_len_bytes_len -bytes_until_end + msg_len;
//SAD CASE
        } else if bytes_until_end <= SZ_OF_USIZE + msg_len {
            self.buffer[tail..tail+SZ_OF_USIZE].copy_from_slice_avx(&msg_len_bytes);

            self.buffer[tail+SZ_OF_USIZE..].copy_from_slice_avx(&msg[..(bytes_until_end-SZ_OF_USIZE)]);
            self.buffer[..msg_len+SZ_OF_USIZE-bytes_until_end].copy_from_slice_avx(&msg[(bytes_until_end-SZ_OF_USIZE)..]);

            *self.tail = msg_len + SZ_OF_USIZE - bytes_until_end;
//HAPPY CASE
        } else {
            self.buffer[tail..tail+SZ_OF_USIZE].copy_from_slice_avx(&msg_len_bytes);
            self.buffer[tail+SZ_OF_USIZE..tail+SZ_OF_USIZE+msg_len].copy_from_slice_avx(msg);
            *self.tail = (tail+SZ_OF_USIZE+msg_len) % self.buffer.len();
        }

        msg_len + SZ_OF_USIZE
    }

    #[cfg(not(feature = "avx2"))]
    pub fn push(&mut self, msg: &[u8]) -> usize {
        //is buffer full?
        //is there room for the message

        let msg_len = msg.len();

        if self.is_full() || msg_len + SZ_OF_USIZE > self.empty_slots_left() {return 0;}

        let tail = *self.tail;
        let msg_len_bytes = msg_len.to_le_bytes();
        let msg_len_bytes_len = msg_len_bytes.len();
        let bytes_until_end = self.buffer.len() - tail;

        if bytes_until_end < SZ_OF_USIZE {
//EXTRA SAD CASE
            self.buffer[tail..].copy_from_slice(&msg_len_bytes[..bytes_until_end]);
            self.buffer[..(msg_len_bytes_len-bytes_until_end)].copy_from_slice(&msg_len_bytes[bytes_until_end..]);
        
            self.buffer[(msg_len_bytes_len-bytes_until_end)..(msg_len_bytes_len-bytes_until_end)+msg_len].copy_from_slice(msg);
            
            *self.tail = msg_len_bytes_len -bytes_until_end + msg_len;
//SAD CASE
        } else if bytes_until_end <= SZ_OF_USIZE + msg_len {
            self.buffer[tail..tail+SZ_OF_USIZE].copy_from_slice(&msg_len_bytes);

            self.buffer[tail+SZ_OF_USIZE..].copy_from_slice(&msg[..(bytes_until_end-SZ_OF_USIZE)]);
            self.buffer[..msg_len+SZ_OF_USIZE-bytes_until_end].copy_from_slice(&msg[(bytes_until_end-SZ_OF_USIZE)..]);

            *self.tail = msg_len + SZ_OF_USIZE - bytes_until_end;
//HAPPY CASE
        } else {
            self.buffer[tail..tail+SZ_OF_USIZE].copy_from_slice(&msg_len_bytes);
            self.buffer[tail+SZ_OF_USIZE..tail+SZ_OF_USIZE+msg_len].copy_from_slice(msg);
            *self.tail = (tail+SZ_OF_USIZE+msg_len) % self.buffer.len();
        }

        msg_len + SZ_OF_USIZE
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
        
        write!(format, "\nRing Buffer: tail: {}, head: {}, size: {}\n [ {} ]\n [ {} ]\n",
                    self.tail,
                    self.head,
                    self.buffer.len(),
                    &hex,
                    &headtail)
    }
}


