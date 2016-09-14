#![feature(test)]

extern crate test;
extern crate crossbeam;
extern crate num_cpus;
extern crate rayon;

use test::Bencher;

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

pub fn vec_bin_op_mut<F>(u: &[f64], v: &[f64], len: usize, dst: &mut [f64], f: &F) -> ()
where F: Fn(f64, f64) -> f64 + Send + Sync + 'static
{
    let mut x_iter = u.iter();
    let mut y_iter = v.iter();

    for dst in dst.iter_mut() {
        *dst = f(*x_iter.next().unwrap(), *y_iter.next().unwrap());
    }
}

pub fn vec_bin_op_2<F>(u: &[f64], v: &[f64], f: &F) -> Vec<f64>
    where F: Fn(f64, f64) -> f64 + Send + Sync + 'static
{
    let len = u.len();
    debug_assert!(len == v.len());

    let mut out_vec = Vec::with_capacity(len);
    unsafe {
        out_vec.set_len(len);
    }

    vec_bin_op_mut(u, v, len, &mut out_vec, f);

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
    let mut chunk_size = (len as f64 / cpus as f64).floor() as usize;
    if len < cpus {
        chunk_size = len;
    }
    return chunk_size;
}


pub fn vec_bin_op_split<F>(u: &[f64], v: &[f64], dst: &mut [f64], chunk_size: &usize, f: &F) -> ()
    where F: Fn(f64, f64) -> f64 + Send + Sync + 'static
{
    debug_assert!(u.len() == v.len());

    if u.len() <= *chunk_size {
        // println!("LEN: {}", u.len());
        vec_bin_op_mut(u, v, u.len(), dst, f);
        return;
    }

    let mid_point = u.len() / 2;
    let (x_left, x_right): (&[f64], &[f64]) = u.split_at(mid_point);
    let (y_left, y_right): (&[f64], &[f64]) = v.split_at(mid_point);
    let (dst_left, dst_right): (&mut [f64], &mut [f64]) = dst.split_at_mut(mid_point);

    rayon::join(|| vec_bin_op_split(x_left, y_left, dst_left, chunk_size, f),
             || vec_bin_op_split(x_right, y_right, dst_right, chunk_size, f));
}

pub fn vec_bin_op_threaded<F>(u: &[f64], v: &[f64], chunk_size: &usize, f: &F) -> Vec<f64>
    where F: Fn(f64, f64) -> f64 + Send + Sync + 'static
{
    let len = u.len();
    debug_assert!(len == v.len());

    let mut out_vec = Vec::with_capacity(len);
    unsafe {
        out_vec.set_len(len);
    }

    vec_bin_op_split(u, v, &mut out_vec, chunk_size, f);

    return out_vec;
}

#[bench]
fn bench_vec_bin_op(b: &mut Bencher) {
    let m = vec![1.; 1600000];
    let m1 = vec![1.; 1600000];
    let f = |x, y| x + y;
    b.iter(|| vec_bin_op(&m, &m1, &f) );
}

#[bench]
fn bench_vec_bin_op_2(b: &mut Bencher) {
    let m = vec![1.; 1600000];
    let m1 = vec![1.; 1600000];
    let f = |x, y| x + y;
    b.iter(|| vec_bin_op_2(&m, &m1, &f) );
}

#[bench]
fn bench_vec_bin_op_mut(b: &mut Bencher) {
    let m = vec![1.; 1600000];
    let m1 = vec![1.; 1600000];
    let f = |x, y| x + y;

    let len = m.len();
    debug_assert!(len == m1.len());

    let mut out_vec = Vec::with_capacity(len);
    unsafe {
        out_vec.set_len(len);
    }

    b.iter(|| vec_bin_op_mut(&m, &m1, len, &mut out_vec, &f) );
}

#[bench]
fn bench_vec_bin_threaded(b: &mut Bencher) {
    let m = vec![1.; 1600000];
    let m1 = vec![1.; 1600000];
    let chunk_size = get_chunk_size(&m, &m1);
    let f = |x, y| x + y;
    b.iter(|| vec_bin_op_threaded(&m, &m1, &chunk_size, &f) );
}

fn main() {
    let m = vec![1.; 16000];
    let m1 = vec![1.; 16000];
    println!("chunk_size: {:?}", get_chunk_size(&m, &m1));
    let f = |x, y| x + y;
    let vm = vec_bin_op(&m, &m1, &f);
    let vtm = vec_bin_op_threaded(&m, &m1, &get_chunk_size(&m, &m1), &f);
    assert!(vm == vtm);
    println!("Calculated correctly!");
}
