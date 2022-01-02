
pub fn print_bitboard(board: u64) {
    let bits = format!("{:064b}", board);
    for row in 0..8 {
        let line = &bits[8 * row..(8 * row + 8)];
        for square in line.chars().rev() {
            match square {
                '0' => print!(". "),
                '1' => print!("1 "),
                _ => unreachable!(),
            }
        }
        println!();
    }
    println!();
}

pub fn square_to_coord(square: u8) -> String {
    let mut result = String::new();
    result.push(match square % 8 {
        0 => 'a',
        1 => 'b',
        2 => 'c',
        3 => 'd',
        4 => 'e',
        5 => 'f',
        6 => 'g',
        7 => 'h',
        _ => unreachable!(),
    });
    result.push(match square / 8 {
        0 => '1',
        1 => '2',
        2 => '3',
        3 => '4',
        4 => '5',
        5 => '6',
        6 => '7',
        7 => '8',
        _ => unreachable!(),
    });

    result
}

pub fn coord_to_square(coord: &str) -> u8 {
    let mut result = match coord.chars().next().unwrap() {
        'a' => 0,
        'b' => 1,
        'c' => 2,
        'd' => 3,
        'e' => 4,
        'f' => 5,
        'g' => 6,
        'h' => 7,
        _ => unreachable!(),
    };
    result += match coord.chars().nth(1).unwrap() {
        '1' => 0,
        '2' => 8,
        '3' => 2 * 8,
        '4' => 3 * 8,
        '5' => 4 * 8,
        '6' => 5 * 8,
        '7' => 6 * 8,
        '8' => 7 * 8,
        _ => unreachable!(),
    };
    result
}

/// flips a square index to the other side of the board
pub fn flip_square(square: usize) -> usize {
    square ^ 56
}