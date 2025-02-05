#[cfg(test)]
mod drain_and_fill_tests{
    use shm_ring::{
            SZ_OF_USIZE, 
            drain_and_fill::drain_and_fill, 
            ringbuffer_ro::RingbufRo, 
            ringbuffer_rw::RingbufRw
    };
    const TEST_SHM_SIZE: usize = 52;//8 for head, 8 for tail, 36 for buffer (35 that are available)

    /// Verifies that an empty reader doesnt do anything
    #[test]
    fn reader_is_empty(){
        let mut buffer: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let mut reader = unsafe{ RingbufRo::new(TEST_SHM_SIZE, buffer.as_mut_ptr()) };
        let mut writer = unsafe{ RingbufRw::new(TEST_SHM_SIZE, buffer.as_mut_ptr()) };

        assert!(reader.is_empty());
        assert!(writer.is_empty());
        
        let amt = drain_and_fill(&mut reader, &mut writer);
        assert_eq!(0, amt);
        
        assert!(reader.is_empty());
        assert!(writer.is_empty());
    }

    /// Verifies that a full writer doesnt do anything
    #[test]
    fn writer_is_full(){
        let msg = b"AAAABBBB";
        let mut buffer1: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let mut buffer2: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let mut reader1 = unsafe{ RingbufRo::new(TEST_SHM_SIZE, buffer1.as_mut_ptr()) };
        let mut writer1 = unsafe{ RingbufRw::new(TEST_SHM_SIZE, buffer1.as_mut_ptr()) };
        let mut _reader2 = unsafe{ RingbufRo::new(TEST_SHM_SIZE, buffer2.as_mut_ptr()) };
        let mut writer2 = unsafe{ RingbufRw::new(TEST_SHM_SIZE, buffer2.as_mut_ptr()) };

        // Put a msg in the reader 
        let _amt = writer1.push(msg);
        assert!(!reader1.is_empty());

        // set the writer to full
        let size = writer2.get_size();
        writer2.set_tail(size - 1);
        assert!(writer2.is_full());

        let amt = drain_and_fill(&mut reader1, &mut writer2);
        assert_eq!(0, amt); // Verify drain_and_fill wrote no bytes
        
        assert!(!reader1.is_empty()); // Verify reader1 is still not empty
        assert!(writer2.is_full()); // Verify writer2 is still full
    }


    /// Verifies that if multiple messages are available, only the subset that will fit get transferred
    /// The reader's buffer is bigger than the writers buffer
    /// 3 msgs are in the reader but only 2 will fit in the writer
    /// expect 1 msg to still be in the reader, 2 in the writer
    #[test]
    fn multiple_msgs_will_fit(){
        let mut buffer1: Vec<u8> = vec![0;TEST_SHM_SIZE*2];
        let mut reader1 = unsafe{ RingbufRo::new(buffer1.len(), buffer1.as_mut_ptr()) };
        let mut writer1 = unsafe{ RingbufRw::new(buffer1.len(), buffer1.as_mut_ptr()) };

        let mut buffer2: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let mut reader2 = unsafe{ RingbufRo::new(TEST_SHM_SIZE, buffer2.as_mut_ptr()) };
        let mut writer2 = unsafe{ RingbufRw::new(TEST_SHM_SIZE, buffer2.as_mut_ptr()) };

        // Put 3 msgs in reader1, only 2 will fit in the writer2 
        let msg = b"AAAABBBB";
        let _amt = writer1.push(msg);
        let _amt = writer1.push(msg);
        let _amt = writer1.push(msg);
        assert!(!reader1.is_empty());
        assert!(writer2.is_empty()); // Verify writer2 is empty

        let amt = drain_and_fill(&mut reader1, &mut writer2);
        assert_eq!((msg.len() + SZ_OF_USIZE)*2, amt); // Verify drain_and_fill wrote the bytes

        assert!(!reader1.is_empty()); // Verify reader1 is NOT empty
        assert!(!writer2.is_empty()); // Verify writer2 is NOT empty

        // Verify the msgs come out writer2
        let mut buffer3: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let amt = reader2.pop(&mut buffer3);
        assert_eq!(msg, &buffer3[..amt]);
        let mut buffer3: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let amt = reader2.pop(&mut buffer3);
        assert_eq!(msg, &buffer3[..amt]);
        assert!(writer2.is_empty());

        // Take the msg that didnt fit in writer2 out of reader1
        let mut buffer3: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let amt = reader1.pop(&mut buffer3);
        assert_eq!(msg, &buffer3[..amt]);
        assert!(reader1.is_empty());

    }


    /// Verifies case 0 where the reader has one part
    /// and writes it to the writer without wrapping
    #[test]
    fn one_part_no_wrap(){
        let mut buffer1: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let mut reader1 = unsafe{ RingbufRo::new(TEST_SHM_SIZE, buffer1.as_mut_ptr()) };
        let mut writer1 = unsafe{ RingbufRw::new(TEST_SHM_SIZE, buffer1.as_mut_ptr()) };

        let mut buffer2: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let mut reader2 = unsafe{ RingbufRo::new(TEST_SHM_SIZE, buffer2.as_mut_ptr()) };
        let mut writer2 = unsafe{ RingbufRw::new(TEST_SHM_SIZE, buffer2.as_mut_ptr()) };

        assert!(reader1.is_empty());
        assert!(writer1.is_empty());

        // Put a msg in the reader 
        let msg = b"AAAABBBB";
        let _amt = writer1.push(msg);
        assert!(!reader1.is_empty());
        // println!("{reader1}");

        let amt = drain_and_fill(&mut reader1, &mut writer2);
        assert_eq!(msg.len() + SZ_OF_USIZE, amt); // Verify drain_and_fill wrote the bytes
        // println!("{writer2}");

        assert!(reader1.is_empty()); // Verify reader1 is empty
        assert!(!writer2.is_empty()); // Verify writer2 is not empty

        let mut buffer3: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let amt = reader2.pop(&mut buffer3);
        assert_eq!(msg, &buffer3[..amt]);
    }

    /// Verifies case 1 where the reader has a message in one part but
    /// wraps when writing to the writer
    #[test]
    fn one_part_wrap(){
        let mut buffer1: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let mut reader1 = unsafe{ RingbufRo::new(TEST_SHM_SIZE, buffer1.as_mut_ptr()) };
        let mut writer1 = unsafe{ RingbufRw::new(TEST_SHM_SIZE, buffer1.as_mut_ptr()) };

        let mut buffer2: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let mut reader2 = unsafe{ RingbufRo::new(TEST_SHM_SIZE, buffer2.as_mut_ptr()) };
        let mut writer2 = unsafe{ RingbufRw::new(TEST_SHM_SIZE, buffer2.as_mut_ptr()) };

        // Advance the head and tail of the writer to 2 bytes before the end to force a wrap on write
        let buffer_end = TEST_SHM_SIZE - 2 * SZ_OF_USIZE - 2;
        reader2.set_head(buffer_end);
        writer2.set_tail(buffer_end);

        assert!(reader2.is_empty());
        assert!(writer2.is_empty());

        // Put a msg in the reader 
        let msg = b"AAAABBBB";
        let _amt = writer1.push(msg);
        assert!(!reader1.is_empty());
        // println!("{reader1}");

        let amt = drain_and_fill(&mut reader1, &mut writer2);
        assert_eq!(msg.len() + SZ_OF_USIZE, amt); // Verify drain_and_fill wrote the bytes
        // println!("{writer2}");

        assert!(reader1.is_empty()); // Verify reader1 is empty
        assert!(!writer2.is_empty()); // Verify writer2 is not empty

        let mut buffer3: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let amt = reader2.pop(&mut buffer3);
        assert_eq!(msg, &buffer3[..amt]);
    }

    /// Verifies case 2 where the reader has 2 parts but they dont wrap in the writer
    #[test]
    fn two_parts_no_wrap(){
        let mut buffer1: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let mut reader1 = unsafe{ RingbufRo::new(TEST_SHM_SIZE, buffer1.as_mut_ptr()) };
        let mut writer1 = unsafe{ RingbufRw::new(TEST_SHM_SIZE, buffer1.as_mut_ptr()) };

        let mut buffer2: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let mut reader2 = unsafe{ RingbufRo::new(TEST_SHM_SIZE, buffer2.as_mut_ptr()) };
        let mut writer2 = unsafe{ RingbufRw::new(TEST_SHM_SIZE, buffer2.as_mut_ptr()) };

        // Advance the head and tail of the reader to 4 bytes before the end to force a msg to wrap
        let buffer_end = TEST_SHM_SIZE - 2 * SZ_OF_USIZE - 4; 
        reader1.set_head(buffer_end);
        writer1.set_tail(buffer_end);
        
        assert!(reader1.is_empty());
        assert!(writer1.is_empty());

        assert!(reader2.is_empty());
        assert!(writer2.is_empty());

        // Put a msg in the reader 
        let msg = b"AAAABBBB";
        let _amt = writer1.push(msg);
        assert!(!reader1.is_empty());
        // println!("{reader1}");

        let amt = drain_and_fill(&mut reader1, &mut writer2);
        assert_eq!(msg.len() + SZ_OF_USIZE, amt); // Verify drain_and_fill wrote the bytes
        // println!("{writer2}");

        assert!(reader1.is_empty()); // Verify reader1 is empty
        assert!(!writer2.is_empty()); // Verify writer2 is not empty

        let mut buffer3: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let amt = reader2.pop(&mut buffer3);
        assert_eq!(msg, &buffer3[..amt]);
    }

    // Verifies case 3 where the reader has 2 parts and the  
    // first part wraps when writing 
    #[test]
    fn two_parts_first_part_wraps(){
        let mut buffer1: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let mut reader1 = unsafe{ RingbufRo::new(TEST_SHM_SIZE, buffer1.as_mut_ptr()) };
        let mut writer1 = unsafe{ RingbufRw::new(TEST_SHM_SIZE, buffer1.as_mut_ptr()) };

        let mut buffer2: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let mut reader2 = unsafe{ RingbufRo::new(TEST_SHM_SIZE, buffer2.as_mut_ptr()) };
        let mut writer2 = unsafe{ RingbufRw::new(TEST_SHM_SIZE, buffer2.as_mut_ptr()) };

        // Advance the head and tail of the reader to 8 bytes before the end to force a msg to wrap
        let buffer_end = TEST_SHM_SIZE - 2 * SZ_OF_USIZE - 8; 
        reader1.set_head(buffer_end);
        writer1.set_tail(buffer_end);
        
        assert!(reader1.is_empty());
        assert!(writer1.is_empty());

        // Advance the head and tail of the reader to 4 bytes before the end to force a msg to wrap
        let buffer_end = TEST_SHM_SIZE - 2 * SZ_OF_USIZE - 4; 
        reader2.set_head(buffer_end);
        writer2.set_tail(buffer_end);

        assert!(reader2.is_empty());
        assert!(writer2.is_empty());

        // Put a msg in the reader 
        let msg = b"AAAABBBB";
        let _amt = writer1.push(msg);
        assert!(!reader1.is_empty());
        // println!("{reader1}");

        let amt = drain_and_fill(&mut reader1, &mut writer2);
        assert_eq!(msg.len() + SZ_OF_USIZE, amt); // Verify drain_and_fill wrote the bytes
        // println!("{writer2}");

        assert!(reader1.is_empty()); // Verify reader1 is empty
        assert!(!writer2.is_empty()); // Verify writer2 is not empty

        let mut buffer3: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let amt = reader2.pop(&mut buffer3);
        assert_eq!(msg, &buffer3[..amt]);
    }

    // Verifies case 4 where the reader has 2 parts and the  
    // second part wraps when writing 
    #[test]
    fn two_parts_second_part_wraps(){
        let mut buffer1: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let mut reader1 = unsafe{ RingbufRo::new(TEST_SHM_SIZE, buffer1.as_mut_ptr()) };
        let mut writer1 = unsafe{ RingbufRw::new(TEST_SHM_SIZE, buffer1.as_mut_ptr()) };

        let mut buffer2: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let mut reader2 = unsafe{ RingbufRo::new(TEST_SHM_SIZE, buffer2.as_mut_ptr()) };
        let mut writer2 = unsafe{ RingbufRw::new(TEST_SHM_SIZE, buffer2.as_mut_ptr()) };

        // Advance the head and tail of the reader to 8 bytes before the end to force a msg to wrap
        let buffer_end = TEST_SHM_SIZE - 2 * SZ_OF_USIZE - 8; 
        reader1.set_head(buffer_end);
        writer1.set_tail(buffer_end);
        
        assert!(reader1.is_empty());
        assert!(writer1.is_empty());

        // Advance the head and tail of the reader to 4 bytes before the end to force a msg to wrap
        let buffer_end = TEST_SHM_SIZE - 2 * SZ_OF_USIZE - 12; 
        reader2.set_head(buffer_end);
        writer2.set_tail(buffer_end);

        assert!(reader2.is_empty());
        assert!(writer2.is_empty());

        // Put a msg in the reader 
        let msg = b"AAAABBBB";
        let _amt = writer1.push(msg);
        assert!(!reader1.is_empty());
        // println!("{reader1}");

        let amt = drain_and_fill(&mut reader1, &mut writer2);
        assert_eq!(msg.len() + SZ_OF_USIZE, amt); // Verify drain_and_fill wrote the bytes
        // println!("{writer2}");

        assert!(reader1.is_empty()); // Verify reader1 is empty
        assert!(!writer2.is_empty()); // Verify writer2 is not empty

        let mut buffer3: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let amt = reader2.pop(&mut buffer3);
        assert_eq!(msg, &buffer3[..amt]);
    }

}