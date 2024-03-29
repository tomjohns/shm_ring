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
        if *self.tail > *self.head {
            *self.tail - *self.head
        } else if *self.tail < *self.head {
            self.size + *self.tail - * self.head
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
        if self.is_full() || msg.len() + size_of::<usize>() > self.empty_slots_left() {return 0;}

        let msg_len_bytes = msg.len().to_le_bytes();

        let bytes_until_end = self.size - *self.tail;

        if bytes_until_end < size_of::<usize>() {
//EXTRA SAD CASE
            self.buffer[*self.tail..].copy_from_slice(&msg_len_bytes[..bytes_until_end]);
            self.buffer[..(msg_len_bytes.len()-bytes_until_end)].copy_from_slice(&msg_len_bytes[bytes_until_end..]);
        

            self.buffer[(msg_len_bytes.len()-bytes_until_end)..(msg_len_bytes.len()-bytes_until_end)+msg.len()].copy_from_slice(msg);
            
            *self.tail = msg_len_bytes.len()-bytes_until_end+msg.len();
        } else if bytes_until_end <= size_of::<usize>() + msg.len() {
//SAD CASE
            self.buffer[*self.tail..*self.tail+size_of::<usize>()].copy_from_slice(&msg_len_bytes);

            self.buffer[*self.tail+size_of::<usize>()..].copy_from_slice(&msg[..(bytes_until_end-size_of::<usize>())]);
            self.buffer[..msg.len()+size_of::<usize>()-bytes_until_end].copy_from_slice(&msg[(bytes_until_end - size_of::<usize>())..]);

            *self.tail = msg.len() + size_of::<usize>() - bytes_until_end;
        } else {
//HAPPY CASE
            self.buffer[*self.tail..*self.tail+size_of::<usize>()].copy_from_slice(&msg_len_bytes);
            self.buffer[*self.tail+size_of::<usize>()..*self.tail+size_of::<usize>()+msg.len()].copy_from_slice(msg);
            *self.tail = (*self.tail+size_of::<usize>()+msg.len()) % self.size;
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


