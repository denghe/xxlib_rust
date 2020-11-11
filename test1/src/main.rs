//mod shared_ptr;

use std::ptr;
use std::mem;
use std::string::String;
use std::time::{Duration, Instant};
use std::io;
use std::rc::*;
use std::cell::*;
use lazy_static::lazy_static;
use std::marker::Sized;
use std::borrow::{BorrowMut, Borrow};
use std::ops::Deref;

fn ZigZagEncode16(i: &i16) -> u16 {
    ((i << 1) ^ (i >> 15)) as u16
}

fn ZigZagEncode32(i: &i32) -> u32 {
    ((i << 1) ^ (i >> 31)) as u32
}

fn ZigZagEncode64(i: &i64) -> u64 {
    ((i << 1) ^ (i >> 63)) as u64
}

fn ZigZagDecode16(i: u16) -> i16 {
    ((i >> 1) as i16) ^ (-((i & 1) as i16))
}

fn ZigZagDecode32(i: u32) -> i32 {
    ((i >> 1) as i32) ^ (-((i & 1) as i32))
}

fn ZigZagDecode64(i: u64) -> i64 {
    ((i >> 1) as i64) ^ (-((i & 1) as i64))
}


#[derive(Debug)]
struct Data {
    buf: Vec<u8>,
    offset: usize,
}


impl Data {
    fn new(cap: usize) -> Data {
        Data {
            buf: Vec::<u8>::with_capacity(cap),
            offset: 0,
        }
    }

    fn Clear(&mut self) {
        self.buf.clear();
        self.offset = 0;
    }

    fn WriteVar64(&mut self, mut v: u64) {
        let mut len = self.buf.len();
        let cap = self.buf.capacity();
        if len + 9 > cap {
            self.buf.reserve((len + 9) * 2);
        }
        let buf = self.buf.as_mut_ptr();
        unsafe {
            while v >= 1 << 7 {
                *buf.offset(len as isize) = ((v as u8) & 0x7fu8) | 0x80u8;
                len += 1;
                v = v >> 7;
            };
            *buf.offset(len as isize) = v as u8;
            len += 1;
            self.buf.set_len(len);
        }
    }

    fn WriteVar32(&mut self, mut v: u32) {
        let mut len = self.buf.len();
        let cap = self.buf.capacity();
        if len + 5 > cap {
            self.buf.reserve((len + 5) * 2);
        }
        let buf = self.buf.as_mut_ptr();
        unsafe {
            while v >= 1 << 7 {
                *buf.offset(len as isize) = ((v as u8) & 0x7fu8) | 0x80u8;
                len += 1;
                v = v >> 7;
            };
            *buf.offset(len as isize) = v as u8;
            len += 1;
            self.buf.set_len(len);
        }
    }

    fn WriteBytes(&mut self, v: *const u8, siz: usize) {
        let len = self.buf.len();
        let cap = self.buf.capacity();
        if len + siz > cap {
            self.buf.reserve((len + siz) * 2);
        }
        let p = self.buf.as_mut_ptr();
        unsafe {
            ptr::copy(v as *const u8, p.offset(len as isize), siz);
            self.buf.set_len(len + siz);
        }
    }

    fn WriteFixed<T: FixedWriter>(&mut self, writer: &T) {
        writer.WriteTo(self)
    }
    fn Write<T: Writer>(&mut self, writer: &T) {
        writer.WriteTo(self)
    }


    fn ReadFixed<T: Reader>(&mut self, reader: &mut T) {
        reader.ReadFrom(self)
    }
}

trait FixedWriter {
    fn WriteTo(&self, d: &mut Data);
}

impl FixedWriter for u8 {
    fn WriteTo(&self, d: &mut Data) {
        d.buf.push(*self);
    }
}

impl FixedWriter for i8 {
    fn WriteTo(&self, d: &mut Data) {
        d.buf.push(*self as u8);
    }
}

impl FixedWriter for bool {
    fn WriteTo(&self, d: &mut Data) {
        if *self == true {
            d.buf.push(1);
        } else {
            d.buf.push(0);
        }
    }
}

macro_rules! MakeFixedWriter {
    ($type:ty)=>{
        impl FixedWriter for $type {
            fn WriteTo(&self, d:&mut Data) {
                d.WriteBytes(self as *const $type as *const u8, mem::size_of::<$type>());
            }
        }
    };
}
MakeFixedWriter!(u16);
MakeFixedWriter!(u32);
MakeFixedWriter!(u64);
MakeFixedWriter!(i16);
MakeFixedWriter!(i32);
MakeFixedWriter!(i64);
MakeFixedWriter!(isize);
MakeFixedWriter!(usize);
MakeFixedWriter!(f32);
MakeFixedWriter!(f64);





trait Writer {
    fn WriteTo(&self, d: &mut Data);
}

impl Writer for u8 {
    fn WriteTo(&self, d: &mut Data) {
        d.WriteFixed(self);
    }
}

impl Writer for i8 {
    fn WriteTo(&self, d: &mut Data) {
        d.WriteFixed(self);
    }
}

impl Writer for bool {
    fn WriteTo(&self, d: &mut Data) {
        d.WriteFixed(self);
    }
}

impl Writer for f32 {
    fn WriteTo(&self, d: &mut Data) {
        d.WriteFixed(self);
    }
}

impl Writer for f64 {
    fn WriteTo(&self, d: &mut Data) {
        d.WriteFixed(self);
    }
}

impl Writer for u16 {
    fn WriteTo(&self, d: &mut Data) {
        d.WriteVar32(*self as u32);
    }
}

impl Writer for u32 {
    fn WriteTo(&self, d: &mut Data) {
        d.WriteVar32(*self as u32);
    }
}

impl Writer for u64 {
    fn WriteTo(&self, d: &mut Data) {
        d.WriteVar64(*self as u64);
    }
}

impl Writer for i16 {
    fn WriteTo(&self, d: &mut Data) {
        d.WriteVar32(ZigZagEncode16(self) as u32);
    }
}

impl Writer for i32 {
    fn WriteTo(&self, d: &mut Data) {
        d.WriteVar32(ZigZagEncode32(self) as u32);
    }
}

impl Writer for i64 {
    fn WriteTo(&self, d: &mut Data) {
        d.WriteVar64(ZigZagEncode64(self) as u64);
    }
}

impl Writer for isize {
    fn WriteTo(&self, d: &mut Data) {
        if mem::size_of::<isize>() == 4 {
            d.WriteVar32(ZigZagEncode32(&(*self as i32)) as u32);
        } else {
            d.WriteVar64(ZigZagEncode64(&(*self as i64)) as u64);
        }
    }
}

impl Writer for usize {
    fn WriteTo(&self, d: &mut Data) {
        if mem::size_of::<usize>() == 4 {
            d.WriteVar32(*self as u32);
        } else {
            d.WriteVar64(*self as u64);
        }
    }
}

impl Writer for &str {
    fn WriteTo(&self, d: &mut Data) {
        let len = self.len();
        d.Write(&len);
        d.WriteBytes(self.as_ptr(), len);
    }
}

impl Writer for String {
    fn WriteTo(&self, d: &mut Data) {
        let len = self.len();
        d.Write(&len);
        d.WriteBytes(self.as_ptr(), len);
    }
}

impl<T: Writer> Writer for Vec<T> {
    fn WriteTo(&self, d: &mut Data) {
        let len = self.len();
        d.Write(&len);
        for o in self {
            d.Write(o);
        }
    }
}


trait Reader {
    fn ReadFrom(&mut self, d: &mut Data);
}

impl Reader for i32 {
    fn ReadFrom(&mut self, d: &mut Data) {
        // todo
    }
}

impl Reader for f32 {
    fn ReadFrom(&mut self, d: &mut Data) {
        // todo
    }
}


#[derive(Debug)]
struct F {
    x: i32,
    y: i32,
}

impl F {
    fn Mut(&self) -> &mut Self {
        unsafe { &mut *(self as *const Self as *mut Self) }
    }
}

// trait MutExt {
//     type RT;
//     fn Mut(&self) -> &mut Self::RT;
// }
// impl<T> MutExt for T {
//     type RT = T;
//     fn Mut(&self) -> &mut T {
//         unsafe { &mut *(self as *const T as *mut T) }
//     }
// }

trait RcMut {
    type RT;
    fn Mut(&self) -> &mut Self::RT;
}
impl<T> RcMut for Rc<T> {
    type RT = T;
    fn Mut(&self) -> &mut T {
        unsafe { &mut *(self.borrow() as *const T as *mut T) }
    }
}


trait OptionRcMut {
    type RT;
    fn Mut(&self) -> Option<&mut Self::RT>;
}
impl<T> OptionRcMut for Option<Rc<T>> {
    type RT = T;
    fn Mut(&self) -> Option<&mut T> {
        if let Some(p) = self {
            return Some(unsafe { &mut *(p.borrow() as *const T as *mut T) });
        }
        None
    }
}
impl<T> OptionRcMut for Option<Weak<T>> {
    type RT = T;
    fn Mut(&self) -> Option<&mut T> {
        if let Some(p) = self {
            let o = p.upgrade();
            if let Some(r) = o {
                return Some(unsafe { &mut *(r.borrow() as *const T as *mut T) });
            }
        }
        None
    }
}



#[derive(Debug)]
struct Node {
    id: i32,
    name: String,
    parent: Option<Weak<Node>>,
    child: Option<Rc<Node>>,
}

macro_rules! BaseOf {
    ($BT:ty, $T:ty) => {
        impl Deref for $T {
            type Target = $BT;
            fn deref(&self) -> &Self::Target {
                &self.base
            }
        }
    };
}


struct A {
    x:i32
}

struct B {
    base:A
}

struct C {
    base:B
}

BaseOf!(A, B);
BaseOf!(B, C);


fn NeedA(a:&A) {
    println!("{}", a.x);
}







#[derive(Debug, Default)]
struct Shared<T> {
    value: RefCell<Option<Rc<T>>>
}
impl<T> Deref for Shared<T> {
    type Target = RefCell<Option<Rc<T>>>;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
impl<T> Shared<T> {
    fn ToMut(&self) -> Option<&mut T> {
        let o = self.value.borrow_mut();
        if let Some(p) = o.as_deref() {
            return Some(unsafe { &mut *(p as *const T as *mut T) });
        }
        return None
    }
}
impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Shared { value : self.value.clone() }
    }
}

struct Foo {
    parent: Shared<Foo>
}

fn main() {
    let foo = Shared::<Foo> { value: RefCell::new(None) };
    if let Some(o) = foo.ToMut() {
        o.parent = foo.clone();
    }
    //foo.v



    // let c = C{ base:B{base:A{x:1}} };
    // println!("{}", c.x);
    // NeedA(&c);

    // let mut n = Rc::new(Node {
    //     id: 1,
    //     name: "p".to_string(),
    //     parent: None,
    //     child: None,
    // });
    //n.child = Some(n);
    // let c = Rc::new(Node {
    //     id: 2,
    //     name: "c".to_string(),
    //     parent: Some( Rc::downgrade(&n) ),
    //     child: None,
    // });
    //n.Mut().child = Some(c);

    //println!("{:?}", n);


    //
    // let f = F { x: 1, y: 2 };
    // println!("{:?}", f);
    // let mf = f.Mut();
    // mf.x = 234;
    // mf.y = 123;
    // println!("{:?}", f);


    //
    // let mut d = Data::new(1024);
    // let mut v: Vec<Vec<Vec<String>>> = Vec::new();
    // let mut v1: Vec<Vec<String>> = Vec::new();
    // let mut v2: Vec<String> = Vec::new();
    //
    // println!("plz input:");
    // let mut i = String::new();
    // io::stdin()
    //     .read_line(&mut i)
    //     .expect("failed to read from stdin");
    //
    // // let trimmed = input_text.trim();
    // // match trimmed.parse::<i32>() {
    // //     Ok(i) => v2.push(i),
    // //     Err(..) => println!("this was not an integer: {}", trimmed),
    // // };
    // v2.push(i);
    // v1.push(v2);
    // v.push(v1);
    //
    // let start = Instant::now();
    // for i in 0..10000000 {
    //     d.Clear();
    //     d.Write(&v);
    // }
    //
    // println!("{:?}", start.elapsed());
    // println!("{:?}", d);
    //
    // // let mut i: i32 = 0;
    // // let mut f: f32 = 0.0;
    // // d.ReadFixed(& mut i);
    // // d.ReadFixed(& mut f);
    // // println!("{} {}", i, f);
}
