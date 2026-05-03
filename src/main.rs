use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::thread;
use std::time::Duration;

struct Test {
    val: Arc<AtomicU8>,
}

impl Test {
    fn bla(&mut self) {
        assert_eq!(self.val.load(Ordering::Acquire), 0);

        let temp_val = self.val.clone();

        self.val.fetch_add(1, Ordering::AcqRel);

        assert_eq!(self.val.load(Ordering::Acquire), 1);

        let join_handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));
            assert_eq!(temp_val.load(Ordering::Acquire), 2);
            temp_val.fetch_add(1, Ordering::AcqRel);
        });

        assert_eq!(self.val.load(Ordering::Acquire), 1);

        self.val.fetch_add(1, Ordering::AcqRel);

        assert_eq!(self.val.load(Ordering::Acquire), 2);

        join_handle.join().unwrap();

        assert_eq!(self.val.load(Ordering::Acquire), 3);
    }
}

fn main() {
    let mut test = Test {
        val: Arc::new(0.into())
    };

    test.bla();
}