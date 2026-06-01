use x11rb::protocol::xproto::Window;

#[derive(Debug, Clone, Copy)]
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
    Window,
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

pub struct LayoutIntent {
    pub mapped: Vec<(Window, Rect)>,
    pub unmapped: Vec<Window>,
}

fn apply_inner_gap(a: &mut Rect, b: &mut Rect, dir: Dir, gap: u32) {
    let g = gap as i32;

    match dir {
        Dir::Horizontal => {
            a.w = a.w.saturating_sub((g / 2) as u32);
            b.x += g / 2;
            b.w = b.w.saturating_sub((g / 2) as u32);
        }
        Dir::Vertical => {
            a.h = a.h.saturating_sub((g / 2) as u32);
            b.y += g / 2;
            b.h = b.h.saturating_sub((g / 2) as u32);
        }
    }
}

impl WMSlot {
    pub fn compute_bounds(&self, bounds: Rect, gap: u32) -> Rect {
        let g = gap as i32;

        Rect {
            x: bounds.x + g,
            y: bounds.y + g,
            w: (bounds.w - 2 * gap),
            h: (bounds.h - 2 * gap),
        }
    }

    fn compute_inner(&self, n: usize, bounds: Rect, gap: u32) -> Vec<Rect> {
        let mut res = Vec::<Rect>::new();
        if n == 1 {
            res.push(bounds);
            return res;
        }

        match self {
            WMSlot::Split {
                dir,
                ratio,
                lhs,
                rhs,
            } => {
                let (mut left, mut right) = bounds.split(*dir, *ratio);

                apply_inner_gap(&mut left, &mut right, *dir, gap);

                let lhs_rects = lhs.compute_inner(n, left, gap);
                let lhs_len = lhs_rects.len();

                res.extend(lhs_rects);
                res.extend(rhs.compute_inner(n - lhs_len, right, gap));
            }
            WMSlot::Stack { dir, slots } => {
                let mut rects = bounds.subdiv(*dir, slots.len() as u32);

                for i in 0..rects.len().saturating_sub(1) {
                    let (a, b) = rects.split_at_mut(i + 1);
                    apply_inner_gap(&mut a[i], &mut b[0], *dir, gap);
                }

                for (slot, rect) in slots.iter().zip(rects) {
                    res.extend(slot.compute_inner(n, rect, gap));
                }
            }
            WMSlot::Drain { dir, take } => {
                let count = take.unwrap_or(n);

                let mut rects = bounds.subdiv(*dir, count as u32);

                for i in 0..rects.len().saturating_sub(1) {
                    let (a, b) = rects.split_at_mut(i + 1);
                    apply_inner_gap(&mut a[i], &mut b[0], *dir, gap);
                }

                res.extend(rects);
            }
            WMSlot::Window => res.push(bounds),
        };

        res
    }

    pub fn compute(&self, n: usize, bounds: Rect, inner_gap: u32, outer_gap: u32) -> Vec<Rect> {
        let bounds = self.compute_bounds(bounds, outer_gap);

        self.compute_inner(n, bounds, inner_gap)
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

        if n == 0 {
            return res;
        }

        match dir {
            Dir::Horizontal => {
                let base_w = self.w / n;
                let remainder = self.w % n;
                for i in 0..n {
                    let extra = if i < remainder { 1 } else { 0 };
                    let w = base_w + extra;
                    let x = self.x + (i * base_w + i.min(remainder)) as i32;
                    res.push(Rect {
                        x,
                        y: self.y,
                        w,
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
                    let y = self.y + (i * base_h + i.min(remainder)) as i32;
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
