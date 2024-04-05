use core::slice;
use std::{mem::{transmute, size_of}, fmt::{Display, Formatter}};

#[cfg(feature = "avx2")]
use crate::avx::SliceExt;

#[derive(Debug)]
pub struct RingbufRo<'a> {
    head : &'a mut usize,
    tail : &'a usize,
    size : usize,
    buffer : &'a [u8],
}

impl <'a> RingbufRo<'a> {
    pub fn make(tail : & 'a usize, head : & 'a mut usize, size : usize, buffer : & 'a [u8]) -> Self {
        Self {
            tail : tail,
            head : head,
            size : size,//TODO: get rid of size, we can get that from buffer.len()
            buffer : buffer,
        }
    }
    
    pub fn new(size : usize, data : * mut u8) -> Self {
        if data.is_null() {panic!("data cannot be null")}
        let tail : & usize = unsafe { transmute(data as * mut usize) };
        let data = unsafe { data.offset(size_of::<usize>() as isize) };
        let head : & mut usize = unsafe { transmute(data as * mut usize) };
        let data = unsafe { data.offset(size_of::<usize>() as isize) };
        let size = size - (size_of::<usize>() * 2);
        return RingbufRo::make(tail, head, size, unsafe {slice::from_raw_parts_mut(data, size)} );
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

    //this function returns the current number of bytes that are in the ring buffer
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
    pub fn pop(&mut self, buffer: &mut [u8]) -> usize{
        //if buffer is empty or there arent enough bytes to fill the msg_len
        if self.is_empty() || self.get_curr_bytes() < size_of::<usize>() {return 0;}

//the msg_len field is wrapping
        if *self.head + size_of::<usize>() > self.size { 
//EXTRA SAD CASE
            let bytes_until_end = self.size - *self.head;
            let first_half = &self.buffer[*self.head..];
            let second_half = &self.buffer[..size_of::<usize>()-bytes_until_end];
            let mut msg_len_bytes: Vec<u8> = vec![];
            msg_len_bytes.extend_from_slice(first_half);
            msg_len_bytes.extend_from_slice(second_half);
            let msg_len = usize::from_le_bytes(msg_len_bytes.try_into().unwrap());
            if msg_len <= self.get_curr_bytes() - size_of::<usize>() { //we've already wrapped so we dont have to worry about the msg wrapping
                unsafe{
                    buffer[..msg_len].copy_from_slice_avx(&self.buffer[size_of::<usize>()-bytes_until_end..msg_len+size_of::<usize>()-bytes_until_end]);
                }  
                *self.head = msg_len + size_of::<usize>() - bytes_until_end;
                msg_len
            } else {
                panic!("dragons afoot");
            } 
        } else {
            //there are at least enough bytes to get the msg_len field
            let msg_len_slice = &self.buffer[*self.head..*self.head+size_of::<usize>()];
            let msg_len= usize::from_le_bytes(msg_len_slice.try_into().unwrap());

            if msg_len <= self.get_curr_bytes() - size_of::<usize>() {
                let bytes_until_end = self.size - *self.head;

                if msg_len > bytes_until_end - size_of::<usize>() { //does the message wrap the buffer
//SAD CASE
                    let first_half = &self.buffer[*self.head+size_of::<usize>()..];
                    let second_half = &self.buffer[..msg_len+size_of::<usize>()-bytes_until_end];
                    unsafe{
                        buffer[..first_half.len()].copy_from_slice_avx(first_half);
                        buffer[first_half.len()..first_half.len()+second_half.len()].copy_from_slice_avx(second_half);
                    }
                    *self.head = msg_len+size_of::<usize>()-bytes_until_end;

                    msg_len
                } else {
//HAPPY CASE
                    unsafe{
                        buffer[..msg_len].copy_from_slice_avx(&self.buffer[*self.head+size_of::<usize>()..*self.head+size_of::<usize>()+msg_len]);
                    }
                    *self.head = (*self.head + size_of::<usize>()+msg_len) % self.size;
                    msg_len
                }
            }else { //there were not enough bytes to fulfil the msg_len, this should never happen
                panic!("dragons afoot");
            }
        }
    }

    #[cfg(not(feature = "avx2"))]
    pub fn pop(&mut self, buffer: &mut [u8]) -> usize{
        //if buffer is empty or there arent enough bytes to fill the msg_len
        let sz_of_usize = size_of::<usize>();
        let curr_bytes = self.get_curr_bytes();

        if self.is_empty() || curr_bytes < sz_of_usize {return 0;}

        let head = *self.head;

//the msg_len field is wrapping
        if head + sz_of_usize > self.size { 
//EXTRA SAD CASE
            let bytes_until_end = self.size - head;
            let first_half = &self.buffer[head..];
            let second_half = &self.buffer[..sz_of_usize-bytes_until_end];
            let mut msg_len_bytes: Vec<u8> = vec![];
            msg_len_bytes.extend_from_slice(first_half);
            msg_len_bytes.extend_from_slice(second_half);
            let msg_len = usize::from_le_bytes(msg_len_bytes.try_into().unwrap());
            if msg_len <= curr_bytes - sz_of_usize { //we've already wrapped so we dont have to worry about the msg wrapping
                buffer[..msg_len].copy_from_slice(&self.buffer[sz_of_usize-bytes_until_end..msg_len+sz_of_usize-bytes_until_end]);
                *self.head = msg_len + sz_of_usize - bytes_until_end;
                msg_len
            } else {
                panic!("dragons afoot");
            } 
        } else {
            //there are at least enough bytes to get the msg_len field
            let msg_len_slice = &self.buffer[head..head+sz_of_usize];
            let msg_len= usize::from_le_bytes(msg_len_slice.try_into().unwrap());

            if msg_len <= curr_bytes - sz_of_usize {
                let bytes_until_end = self.size - head;

                if msg_len > bytes_until_end - sz_of_usize { //does the message wrap the buffer
//SAD CASE
                    let first_half = &self.buffer[head+sz_of_usize..];
                    let second_half = &self.buffer[..msg_len+sz_of_usize-bytes_until_end];
                    buffer[..first_half.len()].copy_from_slice(first_half);
                    buffer[first_half.len()..first_half.len()+second_half.len()].copy_from_slice(second_half);
                    *self.head = msg_len+sz_of_usize-bytes_until_end;

                    msg_len
                } else {
//HAPPY CASE
                    buffer[..msg_len].copy_from_slice(&self.buffer[head+sz_of_usize..head+sz_of_usize+msg_len]);
                    *self.head = (head + sz_of_usize + msg_len) % self.size;
                    msg_len
                }
            }else { //there were not enough bytes to fulfil the msg_len, this should never happen
                panic!("dragons afoot");
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
        
        return write!(format, "\nRing Buffer: tail: {}, head: {}, size: {}\n [ {} ]\n [ {} ]\n",
                      self.tail,
                      self.head,
                      self.size,
                      &hex,
                      &headtail);
    }
}



