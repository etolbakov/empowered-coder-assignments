use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

#[derive(Debug)]
struct Node<T> {
    data: T,
    next: AtomicPtr<Node<T>>,
}

#[derive(Debug)]
pub struct LockFreeQueue<T> {
    head: AtomicPtr<Node<T>>,
    tail: AtomicPtr<Node<T>>
}

impl<T: std::default::Default> LockFreeQueue<T> {

    pub fn new() -> Self {
        let dummy_node = Box::into_raw(Box::new(Node {
            data: Default::default(),
            next: AtomicPtr::new(ptr::null_mut()),
        }));

        LockFreeQueue {
            head: AtomicPtr::new(dummy_node),
            tail: AtomicPtr::new(dummy_node),
        }
    }

    pub fn offer(&self, data: T) {
        let new_node = Box::into_raw(Box::new(Node {
            data,
            next: AtomicPtr::new(ptr::null_mut()),
        }));
        let mut tail = self.tail.load(Ordering::Relaxed);
        let mut next;
        loop {
            unsafe {
                next = (*tail).next.load(Ordering::Relaxed);

                if next.is_null() {
                    if (*tail)
                        .next
                        .compare_exchange(next, new_node, Ordering::Release, Ordering::Relaxed)
                        .unwrap()
                        == next
                    {
                        break;
                    }
                } else {
                    self.tail.compare_exchange(tail, next, Ordering::Release, Ordering::Relaxed);
                    tail = self.tail.load(Ordering::Relaxed);
                }
            }
        }
        self.tail.compare_exchange(tail, new_node, Ordering::Release, Ordering::Relaxed);
    }

    pub fn take(&self) -> Option<T> {
        let mut head = self.head.load(Ordering::Relaxed);
        let mut next;
        loop {
            unsafe {
                next = (*head).next.load(Ordering::Relaxed);

                if next.is_null() {
                    return None;
                }

                if self
                    .head
                    .compare_and_swap(head, next, Ordering::Relaxed)
                    == head
                {
                    let node = Box::from_raw(head);
                    return Some(node.data);
                }

                head = self.head.load(Ordering::Relaxed);
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        let head = self.head.load(Ordering::Relaxed);
        let next = unsafe { (*head).next.load(Ordering::Relaxed) };
        next.is_null()
    }
}

impl<T> Drop for LockFreeQueue<T> {

    fn drop(&mut self) {
        let mut node = self.head.load(Ordering::Relaxed);
        while node != ptr::null_mut() {
            let n = unsafe { Box::from_raw(node) };
            node = n.next.load(Ordering::Relaxed);
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::LockFreeQueue;

    #[test]
    fn test_is_empty() {
        let queue = LockFreeQueue::new();

        assert!(queue.is_empty());

        queue.offer(1);
        assert!(!queue.is_empty());

        queue.take();
        assert!(queue.is_empty());
    }

    #[test]
    fn test_take_from_empty_queue() {
        let queue: LockFreeQueue<i32> = LockFreeQueue::new();
        assert_eq!(queue.take(), None);
    }

    #[test]
    fn test_offer_take() {
        let queue = LockFreeQueue::new();

        queue.offer(1);
        queue.offer(2);

        assert_eq!(queue.take(), Some(0)); // TODO this dummy element
        assert_eq!(queue.take(), Some(1));
        assert_eq!(queue.take(), Some(2));
        assert_eq!(queue.take(), None);
    }
}