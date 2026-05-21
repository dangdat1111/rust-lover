use std::char;

fn main() {
    // primitive data types : int, float, bool, char
    // interger
    // signed (i8,i16,i32,i64,i128,isize)
    // unsigned (u8, u16,u32,u64,u128,usize)
    // Kiểu mặc định: Nếu bạn không khai báo kiểu, Rust sẽ mặc định chọn i32. Đây thường là kiểu nhanh nhất ngay cả trên hệ thống 64-bit.

    let x: i32 = -15;
    println!("{}",x);
    let y : u32 = 15;
    println!("{}", y);
    // sử dụng dấu _ cho dễ đọc
    let money = 1_000_000;
    println!("giá trị của money ={}", money);

    //Mọi biến đều phải được sử dụng, nếu không, compiler sẽ warning. Để skip warning, thêm dấu underscore ở đầu tên biến.
    let _unused_varibale = 3u32;

    let pi: f64 = 3.14;
    println!("pi : {}", pi);

    let is_bool: bool = true;
    println!("is true? {}", is_bool);

    let letter: char = 'a';
    println!("char : {}", letter);

    // compound data type: kiểu dữ liệu phức hợp
    // arrays, tuples, slices, strings

    // arrays
    let numbers: [i32;5] = [1,2,3,4,5];
    println!("Number Array: {:#?}", numbers);

}
