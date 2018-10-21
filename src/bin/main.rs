#[macro_use]
extern crate partitions;

fn main() {
    for mut i in 1u128 ..= 10u128 {
        print!("f({})&={}&", i, i * i - i + 41u128);
        i += 10;
        print!("f({})&={}&", i, i * i - i + 41u128);
        i += 10;
        print!("f({})&={}&", i, i * i - i + 41u128);
        i += 10;
        println!("f({})&={}\\\\", i, i * i - i + 41u128);
    }


    let partition_vec = partition_vec![
        'a' => 0,
        'b' => 1,
        'c' => 0,
        'd' => 1,
        'e' => 2,
    ];

    //println!("{}", partition_vec.capacity());
}
