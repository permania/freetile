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
    pub unmapped: Vec<Window>
}

impl WMSlot {
    pub fn compute(&self, n: usize, bounds: Rect, gap: u32) -> Vec<Rect> {
        let mut res = Vec::<Rect>::new();
	if n == 1 {
	    res.push(bounds);
	    return res
	}

        match self {
            WMSlot::Split { dir, ratio, lhs, rhs } => {
                let (left, right) = bounds.split(*dir, *ratio);
		let lhs_rects = lhs.compute(n, left, gap);
		let lhs_len = lhs_rects.len();
		res.extend(lhs_rects);
                res.extend(rhs.compute(n - lhs_len, right, gap));
            }
            WMSlot::Stack { dir, slots } => {
                let rects = bounds.subdiv(*dir, slots.len() as u32);
                for (slot, rect) in slots.iter().zip(rects) {
                    res.extend(slot.compute(n, rect, gap));
                }
            }
            WMSlot::Drain { dir, take } => {
                let count = take.unwrap_or(n);
                res.extend(bounds.subdiv(*dir, count as u32));
            }
            WMSlot::Window => res.push(bounds),
        };

        res
    }
}

impl Rect {
    pub fn split(&self, dir: Dir, ratio: f64) -> (Rect, Rect) {
        match dir {
            Dir::Horizontal => {
                let lhs_w = (self.w as f64 * ratio) as u32;
                (
                    Rect { x: self.x, y: self.y, w: lhs_w, h: self.h },
                    Rect { x: self.x + lhs_w as i32, y: self.y, w: self.w - lhs_w, h: self.h },
                )
            }
            Dir::Vertical => {
                let lhs_h = (self.h as f64 * ratio) as u32;
                (
                    Rect { x: self.x, y: self.y, w: self.w, h: lhs_h },
                    Rect { x: self.x, y: self.y + lhs_h as i32, w: self.w, h: self.h - lhs_h },
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
                    res.push(Rect { x, y: self.y, w, h: self.h })
                }
            }
            Dir::Vertical => {
                let base_h = self.h / n;
                let remainder = self.h % n;
                for i in 0..n {
                    let extra = if i < remainder { 1 } else { 0 };
                    let h = base_h + extra;
                    let y = self.y + (i * base_h + i.min(remainder)) as i32;
                    res.push(Rect { x: self.x, y, w: self.w, h })
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
        Rect { x: 0, y: 0, w: 1920, h: 1080 }
    }

    // --- split ---

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
    fn split_horizontal_asymmetric() {
        let (lhs, rhs) = screen().split(Dir::Horizontal, 0.75);
        assert_eq!(lhs.w, 1440);
        assert_eq!(rhs.w, 480);
        assert_eq!(lhs.w + rhs.w, screen().w);
    }

    #[test]
    fn split_vertical_asymmetric() {
        let (top, bot) = screen().split(Dir::Vertical, 0.25);
        assert_eq!(top.h, 270);
        assert_eq!(bot.h, 810);
        assert_eq!(top.h + bot.h, screen().h);
    }

    #[test]
    fn split_preserves_y_on_horizontal() {
        let (lhs, rhs) = screen().split(Dir::Horizontal, 0.5);
        assert_eq!(lhs.y, 0);
        assert_eq!(rhs.y, 0);
        assert_eq!(lhs.h, screen().h);
        assert_eq!(rhs.h, screen().h);
    }

    #[test]
    fn split_preserves_x_on_vertical() {
        let (top, bot) = screen().split(Dir::Vertical, 0.5);
        assert_eq!(top.x, 0);
        assert_eq!(bot.x, 0);
        assert_eq!(top.w, screen().w);
        assert_eq!(bot.w, screen().w);
    }

    #[test]
    fn split_horizontal_rhs_starts_where_lhs_ends() {
        let (lhs, rhs) = screen().split(Dir::Horizontal, 0.6);
        assert_eq!(rhs.x, lhs.x + lhs.w as i32);
    }

    #[test]
    fn split_vertical_rhs_starts_where_lhs_ends() {
        let (top, bot) = screen().split(Dir::Vertical, 0.6);
        assert_eq!(bot.y, top.y + top.h as i32);
    }

    #[test]
    fn split_ratio_zero_gives_empty_lhs_horizontal() {
        let (lhs, rhs) = screen().split(Dir::Horizontal, 0.0);
        assert_eq!(lhs.w, 0);
        assert_eq!(rhs.w, screen().w);
    }

    #[test]
    fn split_ratio_one_gives_empty_rhs_horizontal() {
        let (lhs, rhs) = screen().split(Dir::Horizontal, 1.0);
        assert_eq!(lhs.w, screen().w);
        assert_eq!(rhs.w, 0);
    }

    #[test]
    fn split_non_origin_rect_horizontal() {
        let r = Rect { x: 100, y: 50, w: 800, h: 600 };
        let (lhs, rhs) = r.split(Dir::Horizontal, 0.5);
        assert_eq!(lhs.x, 100);
        assert_eq!(rhs.x, 500);
        assert_eq!(lhs.y, 50);
        assert_eq!(rhs.y, 50);
        assert_eq!(lhs.w + rhs.w, 800);
    }

    #[test]
    fn split_non_origin_rect_vertical() {
        let r = Rect { x: 100, y: 50, w: 800, h: 600 };
        let (top, bot) = r.split(Dir::Vertical, 0.5);
        assert_eq!(top.y, 50);
        assert_eq!(bot.y, 350);
        assert_eq!(top.h + bot.h, 600);
    }

    // --- subdiv ---

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
        let rects = screen().subdiv(Dir::Vertical, 7);
        assert_eq!(rects[0].h, 155);
        assert_eq!(rects[1].h, 155);
        assert_eq!(rects[2].h, 154);
    }

    #[test]
    fn subdivide_positions_are_contiguous_vertical() {
        let rects = screen().subdiv(Dir::Vertical, 4);
        for i in 1..rects.len() {
            assert_eq!(rects[i].y, rects[i - 1].y + rects[i - 1].h as i32);
        }
    }

    #[test]
    fn subdivide_positions_are_contiguous_horizontal() {
        let rects = screen().subdiv(Dir::Horizontal, 4);
        for i in 1..rects.len() {
            assert_eq!(rects[i].x, rects[i - 1].x + rects[i - 1].w as i32);
        }
    }

    #[test]
    fn subdiv_count_horizontal() {
        let rects = screen().subdiv(Dir::Horizontal, 5);
        assert_eq!(rects.len(), 5);
    }

    #[test]
    fn subdiv_count_vertical() {
        let rects = screen().subdiv(Dir::Vertical, 3);
        assert_eq!(rects.len(), 3);
    }

    #[test]
    fn subdiv_n1_is_full_rect_horizontal() {
        let rects = screen().subdiv(Dir::Horizontal, 1);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].w, screen().w);
        assert_eq!(rects[0].x, 0);
    }

    #[test]
    fn subdiv_n1_is_full_rect_vertical() {
        let rects = screen().subdiv(Dir::Vertical, 1);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].h, screen().h);
        assert_eq!(rects[0].y, 0);
    }

    #[test]
    fn subdiv_y_and_h_fixed_on_horizontal() {
        let rects = screen().subdiv(Dir::Horizontal, 4);
        for r in rects {
            assert_eq!(r.y, 0);
            assert_eq!(r.h, screen().h);
        }
    }

    #[test]
    fn subdiv_x_and_w_fixed_on_vertical() {
        let rects = screen().subdiv(Dir::Vertical, 4);
        for r in rects {
            assert_eq!(r.x, 0);
            assert_eq!(r.w, screen().w);
        }
    }

    #[test]
    fn subdiv_remainder_first_slots_are_base_plus_one_horizontal() {
        let rects = screen().subdiv(Dir::Horizontal, 7);
        assert_eq!(rects[0].w, 275);
        assert_eq!(rects[1].w, 275);
    }

    #[test]
    fn subdiv_remainder_last_slots_are_base_horizontal() {
        let rects = screen().subdiv(Dir::Horizontal, 7);
        for r in &rects[2..] {
            assert_eq!(r.w, 274);
        }
    }

    #[test]
    fn subdiv_evenly_divisible_all_same_size() {
        let rects = screen().subdiv(Dir::Horizontal, 4);
        for r in &rects {
            assert_eq!(r.w, 480);
        }
    }

    #[test]
    fn subdiv_first_slot_starts_at_origin() {
        let rects = screen().subdiv(Dir::Horizontal, 5);
        assert_eq!(rects[0].x, 0);

        let rects = screen().subdiv(Dir::Vertical, 5);
        assert_eq!(rects[0].y, 0);
    }

    #[test]
    fn subdiv_non_origin_rect_preserves_offset_horizontal() {
        let r = Rect { x: 100, y: 50, w: 800, h: 600 };
        let rects = r.subdiv(Dir::Horizontal, 4);
        assert_eq!(rects.len(), 4);
        assert_eq!(rects[0].x, 100);
        assert_eq!(rects[1].x, 300);
        assert_eq!(rects[2].x, 500);
        assert_eq!(rects[3].x, 700);
        let total: u32 = rects.iter().map(|r| r.w).sum();
        assert_eq!(total, 800);
        for rect in &rects {
            assert_eq!(rect.y, 50);
            assert_eq!(rect.h, 600);
        }
    }

    // --- WMSlot::compute ---

    #[test]
    fn slot_window_returns_bounds() {
        let rects = WMSlot::Window.compute(1, screen(), 0);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].x, 0);
        assert_eq!(rects[0].y, 0);
        assert_eq!(rects[0].w, 1920);
        assert_eq!(rects[0].h, 1080);
    }

    #[test]
    fn slot_split_produces_two_rects() {
        let slot = WMSlot::Split {
            dir: Dir::Horizontal,
            ratio: 0.5,
            lhs: Box::new(WMSlot::Window),
            rhs: Box::new(WMSlot::Window),
        };
        let rects = slot.compute(2, screen(), 0);
        assert_eq!(rects.len(), 2);
    }

    #[test]
    fn slot_split_covers_full_width() {
        let slot = WMSlot::Split {
            dir: Dir::Horizontal,
            ratio: 0.5,
            lhs: Box::new(WMSlot::Window),
            rhs: Box::new(WMSlot::Window),
        };
        let rects = slot.compute(2, screen(), 0);
        let total_w: u32 = rects.iter().map(|r| r.w).sum();
        assert_eq!(total_w, screen().w);
    }

    #[test]
    fn slot_split_no_overlap() {
        let slot = WMSlot::Split {
            dir: Dir::Horizontal,
            ratio: 0.6,
            lhs: Box::new(WMSlot::Window),
            rhs: Box::new(WMSlot::Window),
        };
        let rects = slot.compute(2, screen(), 0);
        assert_eq!(rects[1].x, rects[0].x + rects[0].w as i32);
    }

    #[test]
    fn slot_split_vertical_covers_full_height() {
        let slot = WMSlot::Split {
            dir: Dir::Vertical,
            ratio: 0.5,
            lhs: Box::new(WMSlot::Window),
            rhs: Box::new(WMSlot::Window),
        };
        let rects = slot.compute(2, screen(), 0);
        let total_h: u32 = rects.iter().map(|r| r.h).sum();
        assert_eq!(total_h, screen().h);
    }

    #[test]
    fn slot_stack_produces_correct_count() {
        let slot = WMSlot::Stack {
            dir: Dir::Vertical,
            slots: vec![WMSlot::Window, WMSlot::Window, WMSlot::Window],
        };
        let rects = slot.compute(3, screen(), 0);
        assert_eq!(rects.len(), 3);
    }

    #[test]
    fn slot_stack_covers_full_height() {
        let slot = WMSlot::Stack {
            dir: Dir::Vertical,
            slots: vec![WMSlot::Window, WMSlot::Window, WMSlot::Window],
        };
        let rects = slot.compute(3, screen(), 0);
        let total_h: u32 = rects.iter().map(|r| r.h).sum();
        assert_eq!(total_h, screen().h);
    }

    #[test]
    fn slot_stack_positions_contiguous() {
        let slot = WMSlot::Stack {
            dir: Dir::Vertical,
            slots: vec![WMSlot::Window, WMSlot::Window, WMSlot::Window],
        };
        let rects = slot.compute(3, screen(), 0);
        for i in 1..rects.len() {
            assert_eq!(rects[i].y, rects[i - 1].y + rects[i - 1].h as i32);
        }
    }

    #[test]
    fn slot_drain_none_uses_n() {
        let slot = WMSlot::Drain { dir: Dir::Vertical, take: None };
        let rects = slot.compute(4, screen(), 0);
        assert_eq!(rects.len(), 4);
    }

    #[test]
    fn slot_drain_some_overrides_n() {
        let slot = WMSlot::Drain { dir: Dir::Vertical, take: Some(2) };
        let rects = slot.compute(4, screen(), 0);
        assert_eq!(rects.len(), 2);
    }

    #[test]
    fn slot_drain_covers_full_height() {
        let slot = WMSlot::Drain { dir: Dir::Vertical, take: None };
        let rects = slot.compute(3, screen(), 0);
        let total_h: u32 = rects.iter().map(|r| r.h).sum();
        assert_eq!(total_h, screen().h);
    }

    #[test]
    fn slot_master_stack_layout() {
        // left half is master window, right half drains remaining 3 slaves vertically
        let slot = WMSlot::Split {
            dir: Dir::Horizontal,
            ratio: 0.5,
            lhs: Box::new(WMSlot::Window),
            rhs: Box::new(WMSlot::Drain { dir: Dir::Vertical, take: None }),
        };
        let rects = slot.compute(4, screen(), 0);
        assert_eq!(rects.len(), 4);
        assert_eq!(rects[0].x, 0);
        assert_eq!(rects[0].w, 960);
        for r in &rects[1..] {
            assert_eq!(r.x, 960);
            assert_eq!(r.w, 960);
        }
        let total_slave_h: u32 = rects[1..].iter().map(|r| r.h).sum();
        assert_eq!(total_slave_h, screen().h);
    }
}
