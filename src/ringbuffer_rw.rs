use core::slice;
use std::{mem::{transmute, size_of}, fmt::{Formatter, Display}};
#[cfg(feature = "avx2")]
use crate::avx::SliceExt;

#[derive(Debug)]
pub struct RingbufRw <'a> {
    head : &'a u32,
    tail : &'a mut u32,
    size : u32,
    buffer : &'a mut [u8],
}

impl <'a> RingbufRw <'a> {
    pub fn make(tail : & 'a mut u32, head : & 'a u32, size : u32, buffer : & 'a mut [u8]) -> Self {
        Self {
            tail : tail,
            head : head,
            size : size,//TODO: get rid of size, we can get it from buffer.len()
            buffer : buffer,
        }
    }
    
    pub fn new(size : u32, data : * mut u8) -> Self {
        if data.is_null() {panic!("data cannot be null")}
        let tail : & mut u32 = unsafe { transmute(data as * mut u32) };
        let data = unsafe { data.offset(size_of::<u32>() as isize) };
        let head : & u32 = unsafe { transmute(data as * mut u32) };
        let data = unsafe { data.offset(size_of::<u32>() as isize) };
        let size = size - (size_of::<u32>() * 2) as u32;
        return RingbufRw::make(tail, head, size, unsafe {slice::from_raw_parts_mut(data, size.try_into().unwrap())} );
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
    #[cfg(feature = "avx2")]
    pub fn push(&mut self, msg: &[u8]) -> usize {
        //is buffer full?
        //is there room for the message
        if self.is_full() || msg.len() + size_of::<usize>() > self.empty_slots_left() {return 0;}

        let msg_len_bytes = msg.len().to_le_bytes();

        let bytes_until_end = self.size - *self.tail;

        if bytes_until_end < size_of::<usize>() {
//EXTRA SAD CASE
            unsafe{
                self.buffer[*self.tail..].copy_from_slice_avx(&msg_len_bytes[..bytes_until_end]);
                self.buffer[..(msg_len_bytes.len()-bytes_until_end)].copy_from_slice_avx(&msg_len_bytes[bytes_until_end..]);
            

                self.buffer[(msg_len_bytes.len()-bytes_until_end)..(msg_len_bytes.len()-bytes_until_end)+msg.len()].copy_from_slice_avx(msg);
            }
            *self.tail = msg_len_bytes.len()-bytes_until_end+msg.len();
        } else if bytes_until_end <= size_of::<usize>() + msg.len() {
//SAD CASE
            unsafe{
                self.buffer[*self.tail..*self.tail+size_of::<usize>()].copy_from_slice_avx(&msg_len_bytes);

                self.buffer[*self.tail+size_of::<usize>()..].copy_from_slice_avx(&msg[..(bytes_until_end-size_of::<usize>())]);
                self.buffer[..msg.len()+size_of::<usize>()-bytes_until_end].copy_from_slice_avx(&msg[(bytes_until_end - size_of::<usize>())..]);
            }

            *self.tail = msg.len() + size_of::<usize>() - bytes_until_end;
        } else {
//HAPPY CASE
            unsafe{
                self.buffer[*self.tail..*self.tail+size_of::<usize>()].copy_from_slice_avx(&msg_len_bytes);
                self.buffer[*self.tail+size_of::<usize>()..*self.tail+size_of::<usize>()+msg.len()].copy_from_slice_avx(msg);
            }
            *self.tail = (*self.tail+size_of::<usize>()+msg.len()) % self.size;
        }

        msg.len() + size_of::<usize>()
    }

    #[cfg(not(feature = "avx2"))]
    pub fn push(&mut self, msg: &[u8]) -> usize {
        //is buffer full?
        //is there room for the message
        if self.is_full() || msg.len() as u32 + size_of::<u32>() as u32 > self.empty_slots_left() {return 0;}

        let msg_len_bytes = &msg.len().to_le_bytes()[..size_of::<u32>()];
        dbg!(msg_len_bytes);

        let bytes_until_end = self.size - *self.tail;

        if bytes_until_end < size_of::<u32>() as u32 {
//EXTRA SAD CASE
            self.buffer[*self.tail as usize..].copy_from_slice(&msg_len_bytes[..bytes_until_end as usize]);
            self.buffer[..(msg_len_bytes.len()-bytes_until_end as usize)].copy_from_slice(&msg_len_bytes[bytes_until_end as usize..]);
        

            self.buffer[(msg_len_bytes.len()-bytes_until_end as usize)..(msg_len_bytes.len()-bytes_until_end as usize)+msg.len()].copy_from_slice(msg);
            
            *self.tail = msg_len_bytes.len() as u32-bytes_until_end+msg.len() as u32;
        } else if bytes_until_end <= size_of::<u32>() as u32 + msg.len() as u32 {
//SAD CASE
            self.buffer[*self.tail as usize..*self.tail as usize+size_of::<u32>()].copy_from_slice(&msg_len_bytes);

            self.buffer[*self.tail as usize+size_of::<u32>()..].copy_from_slice(&msg[..(bytes_until_end as usize-size_of::<u32>())]);
            self.buffer[..msg.len()+size_of::<u32>()-bytes_until_end as usize].copy_from_slice(&msg[(bytes_until_end as usize - size_of::<u32>())..]);

            *self.tail = msg.len() as u32+ size_of::<u32>() as u32 - bytes_until_end;
        } else {
//HAPPY CASE
            self.buffer[*self.tail as usize..*self.tail as usize+size_of::<u32>()].copy_from_slice(&msg_len_bytes);
            self.buffer[*self.tail as usize+size_of::<u32>()..*self.tail as usize+size_of::<u32>()+msg.len()].copy_from_slice(msg);
            *self.tail = (*self.tail+size_of::<u32>() as u32+msg.len() as u32) % self.size;
        }

        msg.len() + size_of::<usize>()
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
            } else if i == *self.head as usize{
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


