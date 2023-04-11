use std::{arch::x86_64::*};

pub trait SliceExt<T>{
    unsafe fn copy_from_slice_avx(&mut self, src: &[T]);
}

impl <T: Copy> SliceExt <T> for [T] {
    #[target_feature(enable = "avx2")]
    #[inline(never)]
    unsafe fn copy_from_slice_avx(&mut self, src: &[T])
    where
        T: Copy,
    {
        assert!(self.len() == src.len());

        let mut i = 0;
        let len = src.len();

        while i + 32 <= len {
                let src_ptr = src.as_ptr().add(i);
                let dst_ptr = self.as_mut_ptr().add(i);
            
            unsafe{
                let src_vec = _mm256_loadu_si256(src_ptr as *const __m256i);
                _mm256_storeu_si256(dst_ptr as *mut __m256i, src_vec);
            }
            i += 32;
        }

        while i < len {
            self[i] = src[i];
            i += 1;
        } 
    }
}


#[cfg(test)]
mod tests{
    use std::time::SystemTime;

    use crate::avx::SliceExt;

    #[test]
    fn avx(){
        let now = SystemTime::now();

        let src:Vec<u8> = vec![1;1024*1024*1024*4];
        let mut dst: Vec<u8> = vec![0; 1024*1024*1024*4];
        unsafe {dst.copy_from_slice_avx(&src)};
        // copy_fom_slice_avx(&mut dst, &src);
        // dbg!(&dst); 
        dbg!(now.elapsed().unwrap().as_millis());
    }

    #[test]
    fn no_avx(){
        let now = SystemTime::now();

        let src:Vec<u8> = vec![1;1024*1024*1024*4];
        let mut dst: Vec<u8> = vec![0; 1024*1024*1024*4];
        dst.copy_from_slice(&src);
        // copy_fom_slice_avx(&mut dst, &src);
        // dbg!(&dst); 
        dbg!(now.elapsed().unwrap().as_millis());
    }
}