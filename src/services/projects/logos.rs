//! The logos of the technologies, as pixels: made once from their official vector artwork (Vue,
//! Nuxt and FastAPI from simple-icons, CC0; Topcoat's from its repository; Ferris from
//! rustacean.net, CC0), so the app needs no image library. A row is a string: a space is empty and
//! `a`, `b`... pick a color from the logo's list. The logos belong to their projects. They are used
//! by the menu and by the banner printed when the project starts.

use crate::models::{Backend, Facts, Frontend};

/// A color as red, green and blue.
pub type Rgb = (u8, u8, u8);

/// A logo: rows of pixels (an even number of them, two to a terminal row) and its colors.
pub struct Bitmap {
    pub rows: &'static [&'static str],
    pub colors: &'static [Rgb],
}

/// The marks of a project, small: its frontend's and its backend's, when it has them. A Topcoat
/// frontend is its "T", as its wordmark is not readable small.
pub fn marks(facts: &Facts) -> (Option<&'static Bitmap>, Option<&'static Bitmap>) {
    let front = match facts.frontend {
        Frontend::Nuxt => Some(&NUXT),
        Frontend::Vue => Some(&VUE),
        Frontend::Topcoat => Some(&TOPCOAT_ICON),
        Frontend::None => None,
    };
    let back = match facts.backend {
        Backend::Axum => Some(&FERRIS),
        Backend::Fastapi => Some(&FASTAPI),
        Backend::None => None,
    };
    (front, back)
}

impl Bitmap {
    /// The logo as rows of pixels, `None` where it is empty.
    pub fn pixels(&self) -> Vec<Vec<Option<Rgb>>> {
        self.rows
            .iter()
            .map(|row| {
                row.bytes()
                    .map(|b| {
                        (b != b' ')
                            .then(|| self.colors.get(usize::from(b - b'a')).copied())
                            .flatten()
                    })
                    .collect()
            })
            .collect()
    }
}

/// The pixels shrunk to `height` rows: each new pixel takes the color most of its block has, and
/// is empty when under a third of the block is filled.
pub fn shrink(px: &[Vec<Option<Rgb>>], height: usize) -> Vec<Vec<Option<Rgb>>> {
    let (h, w) = (px.len(), px.first().map_or(0, Vec::len));
    let width = (w * height).div_ceil(h.max(1));
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| {
                    let y0 = y * h / height;
                    let y1 = (((y + 1) * h).div_ceil(height)).clamp(y0 + 1, h);
                    let x0 = x * w / width;
                    let x1 = (((x + 1) * w).div_ceil(width)).clamp(x0 + 1, w);
                    let mut seen: Vec<(Rgb, usize)> = vec![];
                    let mut filled = 0;
                    for row in &px[y0..y1] {
                        for c in row[x0..x1].iter().flatten() {
                            filled += 1;
                            match seen.iter_mut().find(|(k, _)| k == c) {
                                Some((_, n)) => *n += 1,
                                None => seen.push((*c, 1)),
                            }
                        }
                    }
                    let area = (y1 - y0) * (x1 - x0);
                    if filled * 3 >= area {
                        seen.into_iter().max_by_key(|(_, n)| *n).map(|(c, _)| c)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .collect()
}

pub const VUE: Bitmap = Bitmap {
    rows: &[
        "aaaaaaaaaaaa    aaaaaaaaaaaa",
        " aaaaabbbbbba  abbbbbbaaaaa ",
        " aaaaaabbbbba  abbbbbaaaaaa ",
        "  aaaaabbbbbbaabbbbbbaaaaa  ",
        "  aaaaaabbbbbbbbbbbbaaaaa   ",
        "   aaaaaabbbbbbbbbbaaaaaa   ",
        "    aaaaabbbbbbbbbbaaaaa    ",
        "    aaaaaabbbbbbbbaaaaaa    ",
        "     aaaaabbbbbbbbaaaaa     ",
        "     aaaaaabbbbbbaaaaaa     ",
        "      aaaaaabbbbaaaaaa      ",
        "       aaaaabbbbaaaaa       ",
        "       aaaaaabbaaaaaa       ",
        "        aaaaabbaaaaa        ",
        "        aaaaaaaaaaaa        ",
        "         aaaaaaaaaa         ",
        "          aaaaaaaa          ",
        "          aaaaaaaa          ",
        "           aaaaaa           ",
        "           aaaaaa           ",
        "            aaaa            ",
        "            aaa             ",
        "             aa             ",
        "                            ",
    ],
    colors: &[(65, 184, 131), (52, 73, 94)],
};

pub const NUXT: Bitmap = Bitmap {
    rows: &[
        "            aaaa                     ",
        "           aaaaa                     ",
        "           aaaaaa                    ",
        "          aaaaaaa                    ",
        "          aaaaaaaa     aaaa          ",
        "         aaaaaaaaaa    aaaaa         ",
        "        aaaaabbaaaa   aaaaaa         ",
        "        aaaabbbaaaaa aaaaaaaa        ",
        "       aaaaabbbbaaaaaaaaaaaaaa       ",
        "       aaaabbbbbbaaaaaaaaaaaaa       ",
        "      aaaaabbbbbbaaaaaaaa aaaaa      ",
        "     aaaaabbbbbbbbaaaaaa   aaaa      ",
        "     aaaabbbbbbbbbaaaaa    aaaaa     ",
        "    aaaaabbbbbbbbbaaaaa     aaaaa    ",
        "    aaaabbbbbbbbbaaaaa      aaaaa    ",
        "   aaaaabbbbbbbbbaaaa        aaaaa   ",
        "  aaaaabbbbbbbbbaaaaa         aaaa   ",
        "  aaaabbbbbbbbbbaaaa          aaaaa  ",
        " aaaaabbbbbbbbbaaaaa           aaaaa ",
        " aaaabbbbbbbbbaaaaa            aaaaa ",
        "aaaaaaaaaaaaaaaaaa     aaaaaaaaaaaaaa",
        "aaaaaaaaaaaaaaaaa     aaaaaaaaaaaaaaa",
        "aaaaaaaaaaaaaaaa     aaaaaaaaaaaaaaaa",
        " aaaaaaaaaaaaa       aaaaaaaaaaaaaaa ",
    ],
    colors: &[(0, 220, 130), (0, 158, 98)],
};

pub const FASTAPI: Bitmap = Bitmap {
    rows: &[
        "         aaaaaa         ",
        "      aaaaaaaaaaaa      ",
        "     aaaaaaaaaaaaaa     ",
        "   aaaaaaaaaaaaaaaaa    ",
        "   aaaaaaaaaaaaaaaaaa   ",
        "  aaaaaaaaa       aaaa  ",
        " aaaaaaaaaa      aaaaaa ",
        " aaaaaaaaa      aaaaaaa ",
        "aaaaaaaaaa    aaaaaaaaaa",
        "aaaaaaaaa    aaaaaaaaaaa",
        "aaaaaaaaa   aaaaaaaaaaaa",
        "aaaaaaaaa      aaaaaaaaa",
        "aaaaaaaa     aaaaaaaaaaa",
        "aaaaaaaa    aaaaaaaaaaaa",
        "aaaaaaa    aaaaaaaaaaaaa",
        "aaaaaaa  aaaaaaaaaaaaaaa",
        " aaaaa  aaaaaaaaaaaaaaa ",
        " aaaaa aaaaaaaaaaaaaaaa ",
        "  aaaaaaaaaaaaaaaaaaaa  ",
        "   aaaaaaaaaaaaaaaaaa   ",
        "    aaaaaaaaaaaaaaaa    ",
        "     aaaaaaaaaaaaaa     ",
        "      aaaaaaaaaaaa      ",
        "         aaaaaa         ",
    ],
    colors: &[(0, 150, 136)],
};

pub const FERRIS: Bitmap = Bitmap {
    rows: &[
        "               b bb b               ",
        "             bbbbbbbbbbb            ",
        "          bbbbbbbbbbbbbbbb          ",
        "          bbbbbbbbbbbbbbbb      b   ",
        "   b    bbbbbbbbbbbbbbbbbbbb   bb  b",
        "bb bb   bbbbbbbbbbbbbbbbbbbb  bbb bb",
        "bbbbbb bbbbbbbbbbbbbbbbbbbbbbbbbbbb ",
        " bbbb  bbbbbbbbbbbbbbbbbbbbbb  bbbb ",
        "  bbb bbbbbbbbbbabbbbabbbbbbbbbbbb  ",
        "   bb bbbbbbbbbdcabbddcbbbbbbbbbb   ",
        "    bbbbbbbbbbadccbbaacbbbbbbbbb    ",
        "     bbbbbbbbbbcccbbcccbbbbbbbbb    ",
        "    bbbabbbbbbbbabbbbabbbbbbbabbb   ",
        "    bbbbbbbbbbbbbbccbbbbbbbb bbb    ",
        "     bb b   bbbbbbbbabbb    b b     ",
        "      bb b                   bb     ",
        "       bb                    b      ",
        "        b                           ",
    ],
    colors: &[(165, 43, 0), (247, 76, 0), (24, 24, 24), (255, 255, 255)],
};

pub const TOPCOAT: Bitmap = Bitmap {
    rows: &[
        "        aaaaa                                           ",
        "        aaaaa                                           ",
        "                                                        ",
        "aaaaaaaaaaaaa                                           ",
        "aaaaaaaaaaaaa                                           ",
        "aaaaaaaaaaaaa                                           ",
        "      aa                                             aa ",
        "      aa    aaa    aa aa     aaa     aaa     aa aa  a  a",
        "      aa  aaaaaaa  aaaaaa   aaaaaa aaaaaa   aaaaaa aa  a",
        "      aa  aa  aaa  aa  aaa aaa  a aaa  aaa aaa  aa   aa ",
        "      aa aaa   aa  aa   aa aa     aaa   aa aa   aa   aa ",
        "      aa aaa   aa  aa   aa aa     aaa   aa aa   aa   aa ",
        "      aa  aaa aaa  aaa aaa aaaaaa  aa  aaa aaa aaa   aa ",
        "      aa  aaaaaa   aaaaaa   aaaaaa aaaaaa   aaaaaa   aa ",
        "      aa    aaa    aa aa     aaa     aaa     aa aa   aa ",
        "                   aa                                   ",
        "                   aa                                   ",
        "                   aa                                   ",
    ],
    colors: &[(238, 238, 242)],
};

pub const TOPCOAT_ICON: Bitmap = Bitmap {
    rows: &[
        "       aaaaaa",
        "       aaaaaa",
        "             ",
        "aaaaaaaaaaaaa",
        "aaaaaaaaaaaaa",
        "     aa      ",
        "     aa      ",
        "     aa      ",
        "     aa      ",
        "     aa      ",
        "     aa      ",
        "     aa      ",
        "     aa      ",
        "     aa      ",
    ],
    colors: &[(238, 238, 242)],
};
