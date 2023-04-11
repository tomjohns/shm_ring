#[cfg(test)]
mod tests{
    use shm_ring::{ringbuffer_ro::{RingbufRo},ringbuffer_rw::{RingbufRw}};
    const TEST_SHM_SIZE: usize = 52;


    #[test]
    fn test_is_full_fn(){
        let mut buffer: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let r_ring = RingbufRo::new(TEST_SHM_SIZE, buffer.as_mut_ptr());
        let mut w_ring = RingbufRw::new(TEST_SHM_SIZE, buffer.as_mut_ptr());

        let msg = b"AAAABBBBCCCCDDDDEEEEFFFFGGG";
        let _result = w_ring.push(msg);

        assert_eq!(true, r_ring.is_full());
        assert_eq!(true, w_ring.is_full());
        println!("{r_ring}");
        println!("{w_ring}");
    }

    #[test]
    fn test_is_empty_fn(){
        let mut buffer: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let r_ring = RingbufRo::new(TEST_SHM_SIZE, buffer.as_mut_ptr());
        let w_ring = RingbufRw::new(TEST_SHM_SIZE, buffer.as_mut_ptr());

        assert_eq!(true, r_ring.is_empty());
        assert_eq!(true, w_ring.is_empty());
        println!("{r_ring}");
        println!("{w_ring}");
    }

    #[test]
    fn test_push_pop_twice_ring_buffer(){
        let mut buffer: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let mut r_ring = RingbufRo::new(TEST_SHM_SIZE, buffer.as_mut_ptr());
        let mut w_ring = RingbufRw::new(TEST_SHM_SIZE, buffer.as_mut_ptr());

        let msg = b"AAAABB";
        println!("PUSH {:x?}", &msg);
        let result = w_ring.push(msg);
        println!("{w_ring}");
        assert_eq!(14, result);
        assert_eq!(0, w_ring.get_head());
        assert_eq!(14, w_ring.get_tail());
        assert_eq!(36, w_ring.get_size());
        assert_eq!(14, w_ring.get_curr_bytes());
        assert_eq!(21, w_ring.empty_slots_left());
        assert_eq!(false, w_ring.is_empty());
        assert_eq!(false, w_ring.is_full());

        let mut buffer = [0;6];
        println!("POP {:x?}", &msg);
        let sz = r_ring.pop(&mut buffer);
        println!("{r_ring}");
        assert_eq!(msg, &buffer);
        assert_eq!(6, sz);
        assert_eq!(14, r_ring.get_head());
        assert_eq!(14, r_ring.get_tail());
        assert_eq!(36, r_ring.get_size());
        assert_eq!(0, r_ring.get_curr_bytes());
        assert_eq!(35, r_ring.empty_slots_left());
        assert_eq!(true, r_ring.is_empty());
        assert_eq!(false, r_ring.is_full());

        println!("PUSH {:x?}", &msg);
        let result = w_ring.push(msg);
        println!("{w_ring}");
        assert_eq!(14, result);
        assert_eq!(14, w_ring.get_head());
        assert_eq!(28, w_ring.get_tail());
        assert_eq!(36, w_ring.get_size());
        assert_eq!(14, w_ring.get_curr_bytes());
        assert_eq!(21, w_ring.empty_slots_left());
        assert_eq!(false, w_ring.is_empty());
        assert_eq!(false, w_ring.is_full());


        let mut buffer = [0;6];
        println!("POP {:x?}", &msg);
        let sz = r_ring.pop(&mut buffer);
        println!("{r_ring}");
        assert_eq!(msg, &buffer);
        assert_eq!(6, sz);
        assert_eq!(28, r_ring.get_head());
        assert_eq!(28, r_ring.get_tail());
        assert_eq!(36, r_ring.get_size());
        assert_eq!(0, r_ring.get_curr_bytes());
        assert_eq!(35, r_ring.empty_slots_left());
        assert_eq!(true, r_ring.is_empty());
        assert_eq!(false, r_ring.is_full());
    }

    #[test]
    fn test_push_push_full_pop_pop(){
        let mut buffer: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let mut r_ring = RingbufRo::new(TEST_SHM_SIZE, buffer.as_mut_ptr());
        let mut w_ring = RingbufRw::new(TEST_SHM_SIZE, buffer.as_mut_ptr());

        let msg = b"AAAABBBBCC";
        println!("PUSH {:x?}", &msg);
        let _result = w_ring.push(msg);
        println!("{w_ring}");

        let msg = b"AAAABBBBC";
        println!("PUSH {:x?}", &msg);
        let _result = w_ring.push(msg);
        println!("{w_ring}");
        assert_eq!(true, r_ring.is_full());
        assert_eq!(true, w_ring.is_full());


        let mut buffer = [0;10];
        let _result = r_ring.pop(&mut buffer);
        println!("POP {:x?}", &buffer);
        println!("{r_ring}");

        let mut buffer = [0;9];
        let _result = r_ring.pop(&mut buffer);
        println!("POP {:x?}", &buffer);
        println!("{r_ring}");
        assert_eq!(msg, &buffer);
        assert_eq!(true, r_ring.is_empty());
        assert_eq!(true, w_ring.is_empty());

        let msg = b"AAAABBBBCCCCDDDDEEEEFFFFGGG";
        let _result = w_ring.push(msg);
        println!("{w_ring}");
    }

    #[test]
    fn test_happy_wrap(){
        let mut buffer: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let mut r_ring = RingbufRo::new(TEST_SHM_SIZE, buffer.as_mut_ptr());
        let mut w_ring = RingbufRw::new(TEST_SHM_SIZE, buffer.as_mut_ptr());

        let msg = b"AAAA";
        println!("PUSH {:x?}", &msg);
        let _result = w_ring.push(msg);
        println!("{w_ring}");


        let mut buffer = [0;4];
        let _result = r_ring.pop(&mut buffer);
        println!("POP {:x?}", &buffer);
        println!("{r_ring}");
        assert_eq!(msg, &buffer);

        let msg = b"BBBB";
        println!("PUSH {:x?}", &msg);
        let _result = w_ring.push(msg);
        println!("{w_ring}");

        let mut buffer = [0;4];
        let _result = r_ring.pop(&mut buffer);
        println!("POP {:x?}", &buffer);
        println!("{r_ring}");
        assert_eq!(msg, &buffer);

        let msg = b"CCCC";
        println!("PUSH {:x?}", &msg);
        let _result = w_ring.push(msg);
        println!("{w_ring}");

        let mut buffer = [0;4];
        let _result = r_ring.pop(&mut buffer);
        println!("POP {:x?}", &buffer);
        println!("{r_ring}");
        assert_eq!(msg, &buffer);

        let msg = b"DDDD";
        println!("PUSH {:x?}", &msg);
        let _result = w_ring.push(msg);
        println!("{w_ring}");

        let mut buffer = [0;4];
        let _result = r_ring.pop(&mut buffer);
        println!("POP {:x?}", &buffer);
        println!("{r_ring}");
        assert_eq!(msg, &buffer);
    }

    #[test]
    fn test_sad_wrap(){
        let mut buffer: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let mut r_ring = RingbufRo::new(TEST_SHM_SIZE, buffer.as_mut_ptr());
        let mut w_ring = RingbufRw::new(TEST_SHM_SIZE, buffer.as_mut_ptr());

        let msg = b"AAAABB";
        println!("PUSH {:x?}", &msg);
        let _result = w_ring.push(msg);
        println!("{w_ring}");

        let mut buffer = [0;6];
        let _result = r_ring.pop(&mut buffer);
        println!("POP {:x?}", &buffer);
        println!("{r_ring}");
        assert_eq!(msg, &buffer);

        let msg = b"AAAABBBBCCCCDDDDEEEE";
        println!("PUSH {:x?}", &msg);
        let _result = w_ring.push(msg);
        println!("{w_ring}");

        let mut buffer = [0;20];
        let _result = r_ring.pop(&mut buffer);
        println!("POP {:x?}", &buffer);
        println!("{r_ring}");
        assert_eq!(msg, &buffer);

    }

    #[test]
    fn test_extra_sad_wrap(){
        let mut buffer: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let mut r_ring = RingbufRo::new(TEST_SHM_SIZE, buffer.as_mut_ptr());
        let mut w_ring = RingbufRw::new(TEST_SHM_SIZE, buffer.as_mut_ptr());

        let msg = b"AAAABBBBCCCCDDDDEEEEFFFF";
        println!("PUSH {:x?}", &msg);
        let _result = w_ring.push(msg);
        println!("{w_ring}");

        let mut buffer = [0;24];
        let _result = r_ring.pop(&mut buffer);
        println!("POP {:x?}", &buffer);
        println!("{r_ring}");
        assert_eq!(msg, &buffer);

        let msg = b"AAAABBBBCCCCDDDDEEEE";
        println!("PUSH {:x?}", &msg);
        let _result = w_ring.push(msg);
        println!("{w_ring}");

        let mut buffer = [0;20];
        let _result = r_ring.pop(&mut buffer);
        println!("POP {:x?}", &buffer);
        println!("{r_ring}");
        assert_eq!(msg, &buffer);

    }

    #[test]
    fn test_empty_msg(){
        let mut buffer: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let mut r_ring = RingbufRo::new(TEST_SHM_SIZE, buffer.as_mut_ptr());
        let mut w_ring = RingbufRw::new(TEST_SHM_SIZE, buffer.as_mut_ptr());

        let msg = b"";
        println!("PUSH {:x?}", &msg);
        let _result = w_ring.push(msg);
        println!("{w_ring}");

        let mut buffer = [0;0];
        let _result = r_ring.pop(&mut buffer);
        println!("POP {:x?}", &buffer);
        println!("{r_ring}");
        assert_eq!(msg, &buffer);
    }

    #[test]
    fn test_push_msg_when_full(){
        let mut buffer: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let mut w_ring = RingbufRw::new(TEST_SHM_SIZE, buffer.as_mut_ptr());
        let msg = b"AAAABBBBCCCCDDDD";
        println!("PUSH {:x?}", &msg);
        let _result = w_ring.push(msg);
        println!("{w_ring}");
        println!("PUSH {:x?}", &msg);
        let result = w_ring.push(msg);
        println!("{w_ring}");
        assert_eq!(0, result); 
    }

    #[test]
    fn test_pop_msg_when_empty(){
        let mut buffer: Vec<u8> = vec![0;TEST_SHM_SIZE];
        let mut r_ring = RingbufRo::new(TEST_SHM_SIZE, buffer.as_mut_ptr());

        let mut buffer = [0;0];
        let result = r_ring.pop(&mut buffer);
        println!("POP {:x?}", &buffer);
        println!("{r_ring}");
        assert_eq!(0, result);
    }

}