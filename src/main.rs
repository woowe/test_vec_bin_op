#![feature(test)]

extern crate test;
// extern crate crossbeam;
extern crate num_cpus;
extern crate rayon;
// extern crate simple_parallel;

use test::Bencher;

use std::time::Instant;

macro_rules! unroll_by_4 {
    ($ntimes:expr, $e:expr) => {{
        let k = $ntimes;
        for _ in 0..k / 4 {
            $e;$e; $e;$e;
        }
        for _ in 0..k % 4 {
            $e
        }
    }}
}

pub fn vec_bin_op_mut<F>(u: &[f64], v: &[f64], dst: &mut [f64], f: &F) -> ()
where F: Fn(f64, f64) -> f64 + Sync
{
    debug_assert_eq!(u.len(), v.len());
    let len = std::cmp::min(u.len(), v.len());

    let xs = &u[..len];
    let ys = &v[..len];

    {
        let out_slice = &mut dst[..len];

        for i in 0..len {
            out_slice[i] = f(xs[i], ys[i]);
        }
    }
}
pub fn vec_bin_op_2<F>(u: &[f64], v: &[f64], f: &F) -> Vec<f64>
    where F: Fn(f64, f64) -> f64 + Sync
{
    let len = u.len();
    debug_assert!(len == v.len());

    let mut out_vec = Vec::with_capacity(len);
    unsafe {
        out_vec.set_len(len);
    }

    vec_bin_op_mut(u, v, &mut out_vec, f);

    return out_vec;
}

pub fn vec_bin_op<F>(u: &[f64], v: &[f64], f: &F) -> Vec<f64>
    where F: Fn(f64, f64) -> f64
{
    debug_assert_eq!(u.len(), v.len());
    let len = std::cmp::min(u.len(), v.len());

    let xs = &u[..len];
    let ys = &v[..len];

    let mut out_vec = Vec::with_capacity(len);
    unsafe {
        out_vec.set_len(len);
    }

    {
        let out_slice = &mut out_vec[..len];

        for i in 0..len {
            out_slice[i] = f(xs[i], ys[i]);
        }
    }

    out_vec
}


pub fn get_chunk_size<T>(u: &[T], v: &[T]) -> usize {
    debug_assert_eq!(u.len(), v.len());
    let len = std::cmp::min(u.len(), v.len());
    let cpus = num_cpus::get();
    if len <= cpus {
        return len;
    }
    return (len as f64 / cpus as f64).floor() as usize;
}

// pub fn vec_bin_op_split<F>(u: &[f64], v: &[f64], dst: &mut [f64], chunk_size: &usize, f: &F) -> ()
// where F: Fn(f64, f64) -> f64 + Sync
// {
//     debug_assert!(u.len() == v.len());
//     let len = std::cmp::min(u.len(), v.len());
//     if len > *chunk_size {
//         let mid_point = len / 2;
//         assert!(mid_point <= len);
//         let (x_left, x_right): (&[f64], &[f64]) = ( &u[0..mid_point], &u[mid_point..len] );
//         let (y_left, y_right): (&[f64], &[f64]) = ( &v[0..mid_point], &v[mid_point..len] );
//         let (dst_left, dst_right): (&mut [f64], &mut [f64]) = dst.split_at_mut(mid_point);
//
//         simple_parallel::both((x_left, y_left, dst_left), (x_right, y_right, dst_right),
//         move |(x, y, dst)| vec_bin_op_split(x, y, dst, chunk_size, f));
//     }
//     else {
//         vec_bin_op_mut(u, v, dst, f);
//     }
// }

// pub fn vec_bin_op_simpar<F>(u: &[f64], v: &[f64], chunk_size: &usize, f: &F) -> Vec<f64>
//     where F: Fn(f64, f64) -> f64 + Sync
// {
//     debug_assert!(u.len() == v.len());
//     let len = std::cmp::min(u.len(), v.len());
//
//     let mut out_vec = Vec::with_capacity(len);
//     unsafe {
//         out_vec.set_len(len);
//     }
//     {
//         let out_slice = &mut out_vec[..len];
//         vec_bin_op_split_par(u, v, out_slice, chunk_size, f);
//     }
//
//     return out_vec;
// }

pub fn vec_bin_op_split<F>(u: &[f64], v: &[f64], dst: &mut [f64], chunk_size: &usize, f: &F) -> ()
where F: Fn(f64, f64) -> f64 + Sync
{
    debug_assert!(u.len() == v.len());
    let len = std::cmp::min(u.len(), v.len());
    if len > *chunk_size {
        let mid_point = len / 2;
        assert!(mid_point <= len);
        let (x_left, x_right): (&[f64], &[f64]) = ( &u[0..mid_point], &u[mid_point..len] );
        let (y_left, y_right): (&[f64], &[f64]) = ( &v[0..mid_point], &v[mid_point..len] );
        let (dst_left, dst_right): (&mut [f64], &mut [f64]) = dst.split_at_mut(mid_point);

        rayon::join(|| vec_bin_op_split(x_left, y_left, dst_left, chunk_size, f),
        || vec_bin_op_split(x_right, y_right, dst_right, chunk_size, f));
    }
    else {
        // println!("LEN: {}", u.len());
        vec_bin_op_mut(u, v, dst, f);
    }
}


const SPLIT_SIZE: usize = 10000;
pub fn vec_bin_op_threaded<F>(u: &[f64], v: &[f64], chunk_size: &usize, f: &F) -> Vec<f64>
    where F: Fn(f64, f64) -> f64 + Sync
{
    debug_assert!(u.len() == v.len());
    let len = std::cmp::min(u.len(), v.len());

    let mut out_vec = Vec::with_capacity(len);
    unsafe {
        out_vec.set_len(len);
    }
    if *chunk_size > SPLIT_SIZE {
        let out_slice = &mut out_vec[..len];
        vec_bin_op_split(u, v, out_slice, chunk_size, f);
    } else {
        vec_bin_op_mut(u, v, &mut out_vec[..len], f);
    }

    return out_vec;
}

const ARR_SIZE: usize =  8;

#[bench]
fn bench_vec_bin_op(b: &mut Bencher) {
    let m = vec![1.; ARR_SIZE];
    let m1 = vec![1.; ARR_SIZE];
    let f = |x, y| x + y;
    b.iter(|| vec_bin_op(&m, &m1, &f) );
}

// #[bench]
// fn bench_vec_bin_op_2(b: &mut Bencher) {
//     let m = vec![1.; ARR_SIZE];
//     let m1 = vec![1.; ARR_SIZE];
//     let f = |x, y| x + y;
//     b.iter(|| vec_bin_op_2(&m, &m1, &f) );
// }

// #[bench]
// fn bench_vec_bin_op_mut(b: &mut Bencher) {
//     let m = vec![1.; ARR_SIZE];
//     let m1 = vec![1.; ARR_SIZE];
//     let f = |x, y| x + y;
//
//     let len = m.len();
//     debug_assert!(len == m1.len());
//
//     let mut out_vec = Vec::with_capacity(len);
//     unsafe {
//         out_vec.set_len(len);
//     }
//
//     b.iter(|| vec_bin_op_mut(&m, &m1, &mut out_vec, &f) );
// }

#[bench]
fn bench_vec_bin_threaded(b: &mut Bencher) {
    let m = vec![1.; ARR_SIZE];
    let m1 = vec![1.; ARR_SIZE];
    let chunk_size = get_chunk_size(&m, &m1);
    let f = |x, y| x + y;
    b.iter(|| vec_bin_op_threaded(&m, &m1, &chunk_size, &f) );
}

// #[bench]
// fn bench_vec_bin_crossbeam(b: &mut Bencher) {
//     let m = vec![1.; ARR_SIZE];
//     let m1 = vec![1.; ARR_SIZE];
//     let chunk_size = get_chunk_size(&m, &m1);
//     let f = |x, y| x + y;
//     b.iter(|| vec_bin_op_crossbeam(&m, &m1, &128, &f) );
// }

fn main() {
    let m = vec![1.; ARR_SIZE];
    let m1 = vec![1.; ARR_SIZE];
    // println!("chunk_size: {:?}", get_chunk_size(&m, &m1));
    let f = |x: f64, y: f64| x % y * x * y / x;

    let start = Instant::now();
    let vtm = vec_bin_op_threaded(&m, &m1, &get_chunk_size(&m, &m1), &f);
    let dur = Instant::now() - start;
    let threaded_nanos = dur.subsec_nanos() as u64 + dur.as_secs() * 1_000_000_000u64;
    println!("Vec_bin_op_threaded: array size {}: done in {} s", ARR_SIZE, threaded_nanos as f32 / 1e9f32);

    let start = Instant::now();
    let vm = vec_bin_op(&m, &m1, &f);
    let dur = Instant::now() - start;
    let seq_nanos = dur.subsec_nanos() as u64 + dur.as_secs() * 1_000_000_000u64;
    println!("Vec_bin_op: array size {}: done in {} s", ARR_SIZE, seq_nanos as f32 / 1e9f32);

    // let vm = vec_bin_op_crossbeam(&m, &m1, &128, &f);
    assert!(vm == vtm);
    println!("speedup: {:.2}x", seq_nanos as f64 / threaded_nanos as f64);
}
