// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscare-common is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

pub const CHAR_MAX_WIDTH: usize = std::mem::size_of::<char>();

// https://tools.ietf.org/html/rfc3629
pub const fn char_width(b: u8) -> usize {
    const CHAR_WIDTH_MAP: &[usize; 256] = &[
        // 1  2  3  4  5  6  7  8  9  A  B  C  D  E  F
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 0
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 1
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 2
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 3
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 4
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 5
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 6
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 7
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 8
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 9
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // A
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // B
        0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, // C
        2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, // D
        3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, // E
        4, 4, 4, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // F
    ];
    CHAR_WIDTH_MAP[b as usize]
}

pub fn next_valid_char(bytes: &[u8]) -> Option<(usize, char)> {
    let first_byte = *bytes.first()?;
    let char_width = self::char_width(first_byte);
    if (char_width == 0) || (char_width > bytes.len()) {
        return None;
    }

    let mut code = match char_width {
        1 => return Some((1, first_byte as char)),
        2 => (first_byte & 0x1F) as u32,
        3 => (first_byte & 0x0F) as u32,
        4 => (first_byte & 0x07) as u32,
        _ => unreachable!(),
    };

    for &byte in &bytes[1..char_width] {
        if byte & 0xC0 != 0x80 {
            return None;
        }
        code = (code << 6) | (byte & 0x3F) as u32;
    }

    char::from_u32(code).map(|c| (char_width, c))
}

pub fn next_back_valid_char(bytes: &[u8]) -> Option<(usize, char)> {
    let bytes_len = bytes.len();
    let char_width = std::cmp::min(bytes_len, CHAR_MAX_WIDTH);

    let mut index = char_width;
    while index > 0 {
        let char_idx = bytes_len - index;
        let char_bytes = &bytes[char_idx..];

        let first_byte = char_bytes[0];
        if self::char_width(first_byte) == index {
            return self::next_valid_char(char_bytes);
        }
        index -= 1;
    }

    None
}
