use core::slice;
use std::{mem::size_of, fmt::{Display, Formatter}};
use crate::SZ_OF_USIZE;

#[cfg(feature = "avx2")]
use crate::avx::SliceExt;

#[derive(Debug)]
pub struct RingbufRo<'a> {
    pub(crate) head : &'a mut usize,
    pub(crate) tail : &'a usize,
    pub(crate) buffer : &'a [u8],
}

impl <'a> RingbufRo<'a> {
    pub fn make(tail : & 'a usize, head : & 'a mut usize, buffer : & 'a [u8]) -> Self {
        Self { tail, head, buffer }
    }

    /// # Safety
    ///
    /// This function is used to create a ringbuffer from a pointer and length
    /// It is up to the caller to ensure the size argument is correct 
    pub unsafe fn new(size : usize, data : * mut u8) -> Self {
        if data.is_null() {panic!("data cannot be null")}
        let tail : & usize = unsafe { &*(data as * mut usize) };
        let data = unsafe { data.add(SZ_OF_USIZE) };
        let head : & mut usize = unsafe { &mut *(data as * mut usize) };
        let data = unsafe { data.add(SZ_OF_USIZE) };
        let size = size - (size_of::<usize>() * 2);
        return RingbufRo::make(tail, head, unsafe {slice::from_raw_parts_mut(data, size)} );
    }

    pub fn is_empty(&self) -> bool {
        *self.head == *self.tail
    }

    pub fn is_full(&self) -> bool {
        (*self.tail + 1) % self.buffer.len() == *self.head 
    }

    //this function returns the current number of bytes that are in the ring buffer
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
    
    pub fn set_head(&mut self, num: usize) {
        *self.head = num;
    }
    
    pub fn get_tail(&self) -> usize {
        *self.tail
    }

    pub fn get_size(&self) -> usize {
        self.buffer.len()
    }


    pub fn empty_slots_left(&self) -> usize {
        self.buffer.len() - self.get_curr_bytes() - 1
    }

    #[cfg(feature = "avx2")]
    pub fn pop(&mut self, buffer: &mut [u8]) -> usize{
        //if buffer is empty or there arent enough bytes to fill the msg_len
        let curr_bytes = self.get_curr_bytes();

        if self.is_empty() || curr_bytes < SZ_OF_USIZE {return 0;}

        let head = *self.head;

//the msg_len field is wrapping
        if head + SZ_OF_USIZE > self.buffer.len() { 
//EXTRA SAD CASE
            let bytes_until_end = self.buffer.len() - head;
            let first_half = &self.buffer[head..];
            let second_half = &self.buffer[..SZ_OF_USIZE-bytes_until_end];
            let mut msg_len_bytes: [u8;size_of::<usize>()] = [0;size_of::<usize>()];
            msg_len_bytes[..first_half.len()].copy_from_slice(first_half);
            msg_len_bytes[first_half.len()..].copy_from_slice(second_half);
            let msg_len = usize::from_le_bytes(msg_len_bytes);
            if msg_len <= curr_bytes - SZ_OF_USIZE { //we've already wrapped so we dont have to worry about the msg wrapping
                buffer[..msg_len].copy_from_slice_avx(&self.buffer[SZ_OF_USIZE-bytes_until_end..msg_len+SZ_OF_USIZE-bytes_until_end]);
                *self.head = msg_len + SZ_OF_USIZE - bytes_until_end;
                msg_len
            } else {
                panic!("Error: the msg_len is greater than the number of bytes available");
            } 
        } else {
            //there are at least enough bytes to get the msg_len field
            let msg_len_slice = &self.buffer[head..head+SZ_OF_USIZE];
            let msg_len= usize::from_le_bytes(msg_len_slice.try_into().unwrap());

            if msg_len <= curr_bytes - SZ_OF_USIZE {
                let bytes_until_end = self.buffer.len() - head;

                if msg_len > bytes_until_end - SZ_OF_USIZE { //does the message wrap the buffer
//SAD CASE
                    let first_half = &self.buffer[head+SZ_OF_USIZE..];
                    let second_half = &self.buffer[..msg_len+SZ_OF_USIZE-bytes_until_end];
                    buffer[..first_half.len()].copy_from_slice_avx(first_half);
                    buffer[first_half.len()..first_half.len()+second_half.len()].copy_from_slice_avx(second_half);
                    *self.head = msg_len+SZ_OF_USIZE-bytes_until_end;

                    msg_len
                } else {
//HAPPY CASE
                    buffer[..msg_len].copy_from_slice_avx(&self.buffer[head+SZ_OF_USIZE..head+SZ_OF_USIZE+msg_len]);
                    *self.head = (head + SZ_OF_USIZE + msg_len) % self.buffer.len();
                    msg_len
                }
            }else { //there were not enough bytes to fulfil the msg_len, this should never happen
                panic!("Error: not enough bytes to fill msg_len");
            }
        }
    }


    #[cfg(not(feature = "avx2"))]
    pub fn pop(&mut self, buffer: &mut [u8]) -> usize{
        //if buffer is empty or there arent enough bytes to fill the msg_len

        let curr_bytes = self.get_curr_bytes();

        if self.is_empty() || curr_bytes < SZ_OF_USIZE {return 0;}

        let head = *self.head;

//the msg_len field is wrapping
        if head + SZ_OF_USIZE > self.buffer.len() { 
//EXTRA SAD CASE
            let bytes_until_end = self.buffer.len() - head;
            let first_half = &self.buffer[head..];
            let second_half = &self.buffer[..SZ_OF_USIZE-bytes_until_end];
            let mut msg_len_bytes: [u8;SZ_OF_USIZE] = [0;SZ_OF_USIZE];
            msg_len_bytes[..first_half.len()].copy_from_slice(first_half);
            msg_len_bytes[first_half.len()..].copy_from_slice(second_half);
            let msg_len = usize::from_le_bytes(msg_len_bytes);

            if msg_len <= curr_bytes - SZ_OF_USIZE { //we've already wrapped so we dont have to worry about the msg wrapping
                buffer[..msg_len].copy_from_slice(&self.buffer[SZ_OF_USIZE-bytes_until_end..msg_len+SZ_OF_USIZE-bytes_until_end]);
                *self.head = msg_len + SZ_OF_USIZE - bytes_until_end;
                msg_len
            } else {
                panic!("Error: the msg_len is greater than the number of bytes available");
            } 
        } else {
            //there are at least enough bytes to get the msg_len field
            let msg_len_slice = &self.buffer[head..head+SZ_OF_USIZE];
            let msg_len= usize::from_le_bytes(msg_len_slice.try_into().unwrap());

            if msg_len <= curr_bytes - SZ_OF_USIZE {
                let bytes_until_end = self.buffer.len() - head;

                if msg_len > bytes_until_end - SZ_OF_USIZE { //does the message wrap the buffer
//SAD CASE
                    let first_half = &self.buffer[head+SZ_OF_USIZE..];
                    let second_half = &self.buffer[..msg_len+SZ_OF_USIZE-bytes_until_end];
                    buffer[..first_half.len()].copy_from_slice(first_half);
                    buffer[first_half.len()..first_half.len()+second_half.len()].copy_from_slice(second_half);
                    *self.head = msg_len+SZ_OF_USIZE-bytes_until_end;

                    msg_len
                } else {
//HAPPY CASE
                    buffer[..msg_len].copy_from_slice(&self.buffer[head+SZ_OF_USIZE..head+SZ_OF_USIZE+msg_len]);
                    *self.head = (head + SZ_OF_USIZE + msg_len) % self.buffer.len();
                    msg_len
                }
            }else { //there were not enough bytes to fulfil the msg_len, this should never happen
                panic!("Error: not enough bytes to fill msg_len");
            }
        }
    }


}

impl<'a> Display for RingbufRo<'a> {
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



