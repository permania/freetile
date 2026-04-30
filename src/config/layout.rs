use core::borrow;

#[derive(Debug, Clone)]
pub enum Dir {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone)]
pub enum WMSlot {
    Split {
        dir: Dir,
        ratio: f64,
        lhs: Box<WMSlot>,
        rhs: Box<WMSlot>,
    },
    Stack {
        dir: Dir,
        slots: Vec<WMSlot>,
    },
    Drain {
        dir: Dir,
        take: Option<usize>,
    },
    Window {
        head: bool,
    },
}

#[derive(Debug, Clone, Copy)]
struct Rect {
    x: i32,
    y: i32,
    w: u32,
    h: u32,
}

impl WMSlot {
    pub fn compute(&self, n: usize, gap: u32) -> Vec<Rect> {
        let res = Vec::<Rect>::new();

	

        self.compute(n, gap);

        res
    }
}

impl Rect {
    pub fn split(&self, dir: Dir, ratio: f64) -> (Rect, Rect) {
        match dir {
            Dir::Horizontal => {
                let lhs_w = (self.w as f64 * ratio) as u32;

                (
                    Rect {
                        x: self.x,
                        y: self.y,
                        w: lhs_w,
                        h: self.h,
                    },
                    Rect {
                        x: self.x + lhs_w as i32,
                        y: self.y,
                        w: self.w - lhs_w,
                        h: self.h,
                    },
                )
            }

            Dir::Vertical => {
                let lhs_h = (self.h as f64 * ratio) as u32;

                (
                    Rect {
                        x: self.x,
                        y: self.y,
                        w: self.w,
                        h: lhs_h,
                    },
                    Rect {
                        x: self.x,
                        y: self.y + lhs_h as i32,
                        w: self.w,
                        h: self.h - lhs_h,
                    },
                )
            }
        }
    }

    pub fn subdiv(&self, dir: Dir, n: u32) -> Vec<Rect> {
        let mut res = Vec::<Rect>::with_capacity(n as usize);

        match dir {
            Dir::Horizontal => {
                let base_w = self.w / n;
                let remainder = self.w % n;

                for i in 0..n {
                    let extra = if i < remainder { 1 } else { 0 };
                    let w = base_w + extra;
                    let x = (i * base_w + i.min(remainder)) as i32;

                    res.push(Rect {
                        x,
                        y: self.y,
                        w: w,
                        h: self.h,
                    })
                }
            }
            Dir::Vertical => {
                let base_h = self.h / n;
                let remainder = self.h % n;

                for i in 0..n {
                    let extra = if i < remainder { 1 } else { 0 };
                    let h = base_h + extra;
                    let y = (i * base_h + i.min(remainder)) as i32;

                    res.push(Rect {
                        x: self.x,
                        y,
                        w: self.w,
                        h,
                    })
                }
            }
        }

        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen() -> Rect {
        Rect {
            x: 0,
            y: 0,
            w: 1920,
            h: 1080,
        }
    }

    #[test]
    fn split_horizontal_even() {
        let (lhs, rhs) = screen().split(Dir::Horizontal, 0.5);
        assert_eq!(lhs.x, 0);
        assert_eq!(lhs.w, 960);
        assert_eq!(rhs.x, 960);
        assert_eq!(rhs.w, 960);
        assert_eq!(lhs.w + rhs.w, screen().w);
    }

    #[test]
    fn split_vertical_even() {
        let (lhs, rhs) = screen().split(Dir::Vertical, 0.5);
        assert_eq!(lhs.y, 0);
        assert_eq!(lhs.h, 540);
        assert_eq!(rhs.y, 540);
        assert_eq!(rhs.h, 540);
        assert_eq!(lhs.h + rhs.h, screen().h);
    }

    #[test]
    fn split_no_gap() {
        let (lhs, rhs) = screen().split(Dir::Horizontal, 0.6);
        assert_eq!(lhs.w + rhs.w, screen().w);
    }

    #[test]
    fn subdivide_no_gap_horizontal() {
        let rects = screen().subdiv(Dir::Horizontal, 3);
        let total: u32 = rects.iter().map(|r| r.w).sum();
        assert_eq!(total, screen().w);
    }

    #[test]
    fn subdivide_no_gap_vertical() {
        let rects = screen().subdiv(Dir::Vertical, 3);
        let total: u32 = rects.iter().map(|r| r.h).sum();
        assert_eq!(total, screen().h);
    }

    #[test]
    fn subdivide_remainder_distribution() {
        // 1080 / 7 = 154 remainder 2, first 2 should be 155
        let rects = screen().subdiv(Dir::Vertical, 7);
        assert_eq!(rects[0].h, 155);
        assert_eq!(rects[1].h, 155);
        assert_eq!(rects[2].h, 154);
    }

    #[test]
    fn subdivide_positions_are_contiguous() {
        let rects = screen().subdiv(Dir::Vertical, 4);
        for i in 1..rects.len() {
            assert_eq!(rects[i].y, rects[i - 1].y + rects[i - 1].h as i32);
        }
    }
}
