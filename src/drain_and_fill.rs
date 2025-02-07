use crate::ringbuffer_ro::RingbufRo;
use crate::ringbuffer_rw::RingbufRw;
use crate::SZ_OF_USIZE;


/// This function moves messages from the reader to the writer and returns the number of bytes written
pub fn drain_and_fill(reader: &mut RingbufRo, writer: &mut RingbufRw) -> usize {
    //---------------------How much room is there in the writer buffer-----------------------
    let whead = *writer.head;
    let wtail = *writer.tail;

    //calculate how much space is available in the writers ringbuffer
    let free_space = writer.buffer.len() - calc_curr_bytes(whead, wtail, writer.buffer.len()) - 1;
    
    if free_space == 0 {return 0;} //Do i need to do this if it will just result in an empty memcopy?

    let rhead = *reader.head;//this guy is a phantom head that we will use to count messages
    let rtail = *reader.tail;//this guy can be a race condition

    let accumulator = bytes_within_limit(free_space, rhead, rtail, reader.buffer);

    if accumulator == 0 { return 0;}//Do i need to do this if it will just result in an empty memcopy?

    let (first_part, second_part) = get_parts(accumulator, rhead, reader.buffer);

    copy_in_parts(first_part, second_part, wtail, writer.buffer);

//---------------------update tail and head
    *writer.tail = (wtail + accumulator) % writer.buffer.len();
    *reader.head = (rhead + accumulator) % reader.buffer.len();
        
    accumulator
    
}

fn calc_curr_bytes(head: usize, tail: usize, size: usize) -> usize{
        if tail > head {
            tail - head
        } else if tail < head {
            size + tail - head
        } else {
            0
        }
}

///This function returns the length of the next messages payload, excluding the length field [length|payload]
fn peek(head: usize, tail: usize, buffer: &[u8]) -> usize {
    //if buffer is empty or there arent enough bytes to fill the msg_len
    let curr_bytes = calc_curr_bytes(head, tail, buffer.len());

    if curr_bytes < SZ_OF_USIZE {return 0;}//I think this also covers the case when its empty
    // if is_empty(head, tail) || curr_bytes < SZ_OF_USIZE {return 0;}

//the msg_len field is wrapping
    if head + SZ_OF_USIZE > buffer.len() { 
//EXTRA SAD CASE
        let bytes_until_end = buffer.len() - head;
        let first_half = &buffer[head..];
        let second_half = &buffer[..SZ_OF_USIZE-bytes_until_end];
        //Combine the 2 parts into a usize to find the length of the message
        let mut msg_len_bytes: [u8;SZ_OF_USIZE] = [0;SZ_OF_USIZE];
        msg_len_bytes[..first_half.len()].copy_from_slice(first_half);
        msg_len_bytes[first_half.len()..].copy_from_slice(second_half);
        let msg_len = usize::from_le_bytes(msg_len_bytes);

        if msg_len <= curr_bytes - SZ_OF_USIZE { //we've already wrapped so we dont have to worry about the msg wrapping
            msg_len
        } else {
            panic!("Error: the msg_len is greater than the number of bytes available");
        } 
    } else {
        //there are at least enough bytes to get the msg_len field
        let msg_len_slice = &buffer[head..head+SZ_OF_USIZE];
        let msg_len= usize::from_le_bytes(msg_len_slice.try_into().unwrap());

        if msg_len <= curr_bytes - SZ_OF_USIZE {
            msg_len
        }else { //there were not enough bytes to fulfil the msg_len, this should never happen
            panic!("Error: not enough bytes to fill msg_len");
        }
    }
}

/// This function calculates the largest number of bytes within the ringbuffer for a given head and tail that fits within a limit, 
/// quantized by whole message boundaries
fn bytes_within_limit(limit: usize, mut phantom_head: usize, phantom_tail: usize, buffer: &[u8]) -> usize {
//---------------------Whats the largest "contiguous" array of whole messages that will fit in the writer
    let mut accumulator: usize = 0;//what if there arent enough messages?
    while accumulator <= limit {
        let next_msg_len = peek(phantom_head, phantom_tail, buffer) + SZ_OF_USIZE;
        if next_msg_len - SZ_OF_USIZE == 0 || accumulator + next_msg_len > limit {//No more messages or too many to fit
            break;
        }
        accumulator += next_msg_len;
        phantom_head = (phantom_head + next_msg_len) % buffer.len();
    }

    accumulator
}

/// given a pre calculated contiguous region of messages (accumulator), get references to them
fn get_parts(accumulator: usize, phantom_head: usize, buffer: &[u8]) -> (&[u8], Option<&[u8]>){
    //---------------------Get the contiguous messages in potentially 2 parts
    let first_part: &[u8];
    let mut second_part: Option<&[u8]> = None;
    let bytes_until_end = buffer.len() - phantom_head;
    if phantom_head + accumulator > bytes_until_end{//split the messages into two parts
        first_part = &buffer[phantom_head..];
        second_part = Some(&buffer[..accumulator-bytes_until_end]);
    } else {
        first_part = &buffer[phantom_head..phantom_head+accumulator]
    }
    
    (first_part, second_part)
}

fn copy_in_parts(first_part: &[u8], second_part: Option<&[u8]>, phantom_tail: usize, buffer: &mut [u8]){
    let bytes_until_end = buffer.len() - phantom_tail;

    let first_part_len = first_part.len();
    let new_phantom_tail = if first_part_len <= bytes_until_end { // does first part wrap
        buffer[phantom_tail..phantom_tail+first_part_len].copy_from_slice(first_part);
        phantom_tail + first_part_len
    } else {
        buffer[phantom_tail..].copy_from_slice(&first_part[..bytes_until_end]);
        buffer[..first_part_len-bytes_until_end].copy_from_slice(&first_part[bytes_until_end..]);
        first_part_len - bytes_until_end
    }; 

    if let Some(second_part) = second_part {
        let bytes_until_end = buffer.len() - new_phantom_tail;
        let second_part_len = second_part.len();
        if second_part_len <= bytes_until_end{ //No wrap
            buffer[new_phantom_tail..new_phantom_tail+second_part_len].copy_from_slice(second_part);
        } else {
            buffer[new_phantom_tail..].copy_from_slice(&second_part[..bytes_until_end]);
            buffer[..second_part_len-bytes_until_end].copy_from_slice(&second_part[bytes_until_end..]);
        }
    }
}