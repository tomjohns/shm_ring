use core::slice;
use std::{mem::{transmute, size_of}, fmt::{Display, Formatter}};

#[cfg(feature = "avx2")]
use crate::avx::SliceExt;

#[derive(Debug)]
pub struct RingbufRo<'a> {
    head : &'a mut u32,
    tail : &'a u32,
    size : u32,
    buffer : &'a [u8],
}

impl <'a> RingbufRo<'a> {
    pub fn make(tail : & 'a u32, head : & 'a mut u32, size : u32, buffer : & 'a [u8]) -> Self {
        Self {
            tail : tail,
            head : head,
            size : size,//TODO: get rid of size, we can get that from buffer.len()
            buffer : buffer,
        }
    }
    
    pub fn new(size : u32, data : * mut u8) -> Self {
        if data.is_null() {panic!("data cannot be null")}
        let tail : & u32 = unsafe { transmute(data as * mut u32) };
        let data = unsafe { data.offset(size_of::<u32>() as isize) };
        let head : & mut u32 = unsafe { transmute(data as * mut u32) };
        let data = unsafe { data.offset(size_of::<u32>() as isize) };
        let size: u32 = size - (size_of::<u32>() * 2) as u32;
        return RingbufRo::make(tail, head, size, unsafe {slice::from_raw_parts_mut(data, size.try_into().unwrap())} );
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

    pub fn get_curr_bytes(&self) -> u32 {
        if *self.tail > *self.head {
            *self.tail - *self.head
        } else if *self.tail < *self.head {
            self.size + *self.tail - * self.head
        } else {
            0
        }
    }

    pub fn get_head(&self) -> u32 {
        *self.head
    }
    
    pub fn get_tail(&self) -> u32 {
        *self.tail
    }

    pub fn get_size(&self) -> u32 {
        self.size
    }

    pub fn empty_slots_left(&self) -> u32 {
        self.size - self.get_curr_bytes() - 1
    }

    // pub fn peek_sz(&self) -> usize {
    //     if self.is_empty() || self.get_curr_bytes() < size_of::<usize>() {return 0;}
    //     if *self.head + size_of::<usize>() > self.size { 
    //         let bytes_until_end = self.size - *self.head;
    //         let first_half = &self.buffer[*self.head..];
    //         let second_half = &self.buffer[..size_of::<usize>()-bytes_until_end];
    //         let mut msg_len_bytes: Vec<u8> = vec![];
    //         msg_len_bytes.extend_from_slice(first_half);
    //         msg_len_bytes.extend_from_slice(second_half);
            
    //         usize::from_le_bytes(msg_len_bytes.try_into().unwrap())
    //     } else {
    //         let msg_len_slice = &self.buffer[*self.head..*self.head+size_of::<usize>()];
    //         usize::from_le_bytes(msg_len_slice.try_into().unwrap())
    //     }
    // }
    #[cfg(feature = "avx2")]
    pub fn pop(&mut self, buffer: &mut [u8]) -> u32{
        //if buffer is empty or there arent enough bytes to fill the msg_len
        if self.is_empty() || self.get_curr_bytes() < size_of::<u32>().try_into().unwrap() {return 0;}

//the msg_len field is wrapping
        if *self.head + size_of::<u32>() as u32 > self.size { 
//EXTRA SAD CASE
            let bytes_until_end = self.size - *self.head;
            let first_half = &self.buffer[*self.head as usize..];
            let second_half = &self.buffer[..size_of::<usize>()-bytes_until_end as usize];
            let mut msg_len_bytes: Vec<u8> = vec![];
            msg_len_bytes.extend_from_slice(first_half);
            msg_len_bytes.extend_from_slice(second_half);
            let msg_len = u32::from_le_bytes(msg_len_bytes.try_into().unwrap());
            if msg_len <= self.get_curr_bytes() - size_of::<u32>() as u32{ //we've already wrapped so we dont have to worry about the msg wrapping
                buffer[..msg_len as usize].copy_from_slice_avx(&self.buffer[size_of::<u32>()-bytes_until_end as usize..msg_len as usize+size_of::<u32>()-bytes_until_end as usize]);
                *self.head = msg_len + size_of::<u32>() as u32 - bytes_until_end;
                msg_len
            } else {
                panic!("dragons afoot");
            } 
        } else {
            //there are at least enough bytes to get the msg_len field
            let msg_len_slice = &self.buffer[*self.head as usize..*self.head as usize+size_of::<u32>()];
            let msg_len= u32::from_le_bytes(msg_len_slice.try_into().unwrap());

            if msg_len <= self.get_curr_bytes() - size_of::<u32>() as u32 {
                let bytes_until_end = self.size - *self.head;

                if msg_len > bytes_until_end - size_of::<u32>() as u32 { //does the message wrap the buffer
//SAD CASE
                    let first_half = &self.buffer[*self.head as usize+size_of::<u32>()..];
                    let second_half = &self.buffer[..msg_len as usize+size_of::<u32>()-bytes_until_end as usize];
                    buffer[..first_half.len()].copy_from_slice_avx(first_half);
                    buffer[first_half.len()..first_half.len()+second_half.len()].copy_from_slice_avx(second_half);
                    *self.head = msg_len+size_of::<u32>() as u32-bytes_until_end;

                    msg_len
                } else {
//HAPPY CASE
                    buffer[..msg_len.try_into().unwrap()].copy_from_slice_avx(&self.buffer[*self.head as usize+size_of::<u32>()..*self.head as usize+size_of::<u32>()+msg_len as usize]);
                    *self.head = (*self.head + size_of::<u32>() as u32+msg_len) % self.size;
                    msg_len
                }
            }else { //there were not enough bytes to fulfil the msg_len, this should never happen
                panic!("dragons afoot");
            }
        }
    }

    #[cfg(not(feature = "avx2"))]
    pub fn pop(&mut self, buffer: &mut [u8]) -> u32{
        //if buffer is empty or there arent enough bytes to fill the msg_len
        if self.is_empty() || self.get_curr_bytes() < size_of::<u32>().try_into().unwrap() {return 0;}

//the msg_len field is wrapping
        if *self.head + size_of::<u32>() as u32 > self.size { 
//EXTRA SAD CASE
            let bytes_until_end = self.size - *self.head;
            let first_half = &self.buffer[*self.head as usize..];
            let second_half = &self.buffer[..size_of::<usize>()-bytes_until_end as usize];
            let mut msg_len_bytes: Vec<u8> = vec![];
            msg_len_bytes.extend_from_slice(first_half);
            msg_len_bytes.extend_from_slice(second_half);
            let msg_len = u32::from_le_bytes(msg_len_bytes.try_into().unwrap());
            if msg_len <= self.get_curr_bytes() - size_of::<u32>() as u32{ //we've already wrapped so we dont have to worry about the msg wrapping
                buffer[..msg_len as usize].copy_from_slice(&self.buffer[size_of::<u32>()-bytes_until_end as usize..msg_len as usize+size_of::<u32>()-bytes_until_end as usize]);
                *self.head = msg_len + size_of::<u32>() as u32 - bytes_until_end;
                msg_len
            } else {
                panic!("dragons afoot");
            } 
        } else {
            //there are at least enough bytes to get the msg_len field
            let msg_len_slice = &self.buffer[*self.head as usize..*self.head as usize+size_of::<u32>()];
            let msg_len= u32::from_le_bytes(msg_len_slice.try_into().unwrap());

            if msg_len <= self.get_curr_bytes() - size_of::<u32>() as u32 {
                let bytes_until_end = self.size - *self.head;

                if msg_len > bytes_until_end - size_of::<u32>() as u32 { //does the message wrap the buffer
//SAD CASE
                    let first_half = &self.buffer[*self.head as usize+size_of::<u32>()..];
                    let second_half = &self.buffer[..msg_len as usize+size_of::<u32>()-bytes_until_end as usize];
                    buffer[..first_half.len()].copy_from_slice(first_half);
                    buffer[first_half.len()..first_half.len()+second_half.len()].copy_from_slice(second_half);
                    *self.head = msg_len+size_of::<u32>() as u32-bytes_until_end;

                    msg_len
                } else {
//HAPPY CASE
                    buffer[..msg_len.try_into().unwrap()].copy_from_slice(&self.buffer[*self.head as usize+size_of::<u32>()..*self.head as usize+size_of::<u32>()+msg_len as usize]);
                    *self.head = (*self.head + size_of::<u32>() as u32+msg_len) % self.size;
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
            } else if i == *self.head as usize {
                String::from("HEAD^")
            } else if i == *self.tail as usize{
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



