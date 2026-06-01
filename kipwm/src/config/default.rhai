fn wrapped_master(n) {
    master(n)
}

// TODO: remove this when done testing
fn ratiotile(n) {
    if n <= 1 {
        return window();
    }

    let cols = (n.to_float()).sqrt().ceiling().to_int();
    let rows = (n + cols - 1) / cols;

    let row_slots = [];

    for row in 0..rows {
        let start = row * cols;
        let end = min(start + cols, n);
        let count = end - start;

        let row_slot = if count == 1 {
            window()
        } else {
            let wins = [];
            for i in 0..count {
                wins.push(window());
            }
            stack(h(), wins)
        };

        row_slots.push(row_slot);
    }

    if rows == 1 {
        row_slots[0]
    } else {
        stack(v(), row_slots)
    }
}
