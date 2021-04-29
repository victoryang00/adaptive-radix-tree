use std::ptr::null_mut;
use std::cmp::min;
use std::borrow::Borrow;

const MAX_PREFIX_LEN: u32 = 10;

type ArtError = u32;
type Result<T> = std::result::Result<T, ArtError>;

enum Node{
    Leaf{
        leaf: ArtNodeLeaf,
    },
    Internal{
        internal: ArtNodeInternal,
    }
}

#[derive(Copy, Clone)]
struct InternalNodeHeader {
    partial_len: u32,
    num_children: u8,
    partial: [u8; MAX_PREFIX_LEN as usize],
}

struct ArtNodeLeaf{
    value: u32, // TODO: make arbitrary
    key_len: u32,
    key: Box<[u8]>,
}

struct ArtNodeInternal{
    header: InternalNodeHeader,
    inner: ArtNodeInternalInner,
}

enum ArtNodeInternalInner {
    Node4 {
        keys: [u8;4],
        children: [MyNode;4],
    },
    Node16 {
        keys: [u8;16],
        children: [MyNode;16],
    },
    Node48 {
        keys: [u8;256],
        children: [MyNode;48],
    },
    Node256 {
        children: [MyNode;256],
    },
}

type MyNode = Option<Box<Node>>;

pub struct ArtTree{
    root: MyNode,
    size: u64,
}

impl ArtTree{
    pub fn new() -> Self{
        Self{
            root: None,
            size: 0
        }
    }


    /// Searches for a value in the ART tree
    /// @arg t The tree
    /// @arg key The key
    /// @arg key_len The length of the key
    /// @return NULL if the item was not found, otherwise
    /// the value pointer is returned.
    pub fn search(&mut self, key: &[u8], key_len: u32) -> Option<u32>{
        let mut n_iter = self.root.as_mut();
        let mut depth = 0;
        while let Some(node) = n_iter {
            match **node {
                Node::Leaf { ref leaf} => {
                    if leaf.leaf_matches(key, key_len, depth) {
                        return Some(leaf.value);
                    }
                    return None;
                }
                Node::Internal { ref mut internal } => {
                    let header = internal.header;

                    if header.partial_len != 0 {

                        let prefix_len = header.check_prefix( key, key_len, depth);
                        if prefix_len != min(MAX_PREFIX_LEN, header.partial_len) {
                            return None;
                        }
                        depth = depth + header.partial_len;
                    }

                    n_iter = internal.find_child(key[depth as usize]);
                    depth+=1;
                }
            }
        }
        None
    }

    pub fn minimum(&mut self) -> Option<&mut ArtNodeLeaf>{
        match &mut self.root {
            None => None,
            Some(node) => node.minimum(),
        }
    }

    pub fn maximum(&mut self) -> Option<&mut ArtNodeLeaf>{
        match &mut self.root {
            None => None,
            Some(node) => node.maximum(),
        }
    }
}

impl Node{
    fn minimum(&mut self) -> Option<&mut ArtNodeLeaf> {
        match self{
            Node::Leaf { leaf } => Some(leaf),
            Node::Internal { internal } => internal.minimum(),
        }
    }

    fn maximum(&mut self) -> Option<&mut ArtNodeLeaf> {
        match self{
            Node::Leaf { leaf } => Some(leaf),
            Node::Internal { internal } => internal.maximum(),
        }
    }
}

impl ArtNodeInternal {
    fn find_child(&mut self, c: u8) -> Option<&mut Box<Node>> {
        let n = self.header;
        match &mut self.inner {
            ArtNodeInternalInner::Node4 { keys, children, .. } => {
                for i in 0..4 {
                    if keys[i] == c {
                        return children[i].as_mut();
                    }
                }
            }
            ArtNodeInternalInner::Node16 { keys, children } => {
                for i in 0..min(16, n.num_children) {
                    if keys[i as usize] == c {
                        return children[i as usize].as_mut();
                    }
                }
            }
            ArtNodeInternalInner::Node48 { keys, children } => {
                let i = keys[c as usize];
                if i != 0 {
                    return children[(i - 1) as usize].as_mut();
                }
            }
            ArtNodeInternalInner::Node256 { children } => {
                return children[c as usize].as_mut();
            }
        }
        return None;
    }

    /*
    fn expand_if_needed(self) -> Self{
        match self{
            ArtNodeInternal::Node4 { n, keys, mut children } if n.num_children == 4 => {
                let mut keys_new = [0;16];
                const INIT: MyNode = None;
                let mut children_new = [INIT; 16];
                for i in 0..4{
                    keys_new[i] = keys[i];
                    children_new[i] = children[i].take();
                }
                ArtNodeInternal::Node16{
                    n,
                    keys: keys_new,
                    children: children_new,
                }
            },
            v => v
        }
    }

     */

    fn add_child(&mut self, c: u8, child: MyNode) {
        let n = &mut self.header;

        match &mut self.inner {
            ArtNodeInternalInner::Node4 { keys, children } => {
                if n.num_children < 4 {
                    let m = n.num_children as usize;
                    let idx = keys.iter().position(|&key| c < key).unwrap_or(m);
                    //keys.copy_within(idx..m, idx+1);
                    for i in (idx..m).rev() {
                        keys[i] = keys[i - 1];
                        children[i] = children[i - 1].take();
                    }

                    keys[idx] = c;
                    children[idx] = child;
                    n.num_children += 1;
                } else {
                    const INIT: MyNode = None;
                    let mut children_new: [MyNode; 16] = [INIT; 16];
                    let mut keys_new: [u8; 16] = [0; 16];
                    for i in 0..4 {
                        keys_new[i] = keys[i];
                        children_new[i] = children[i].take();
                    }

                    self.inner = ArtNodeInternalInner::Node16 {
                        keys: keys_new,
                        children: children_new,
                    };
                    self.add_child(c, child);
                }
            }
            ArtNodeInternalInner::Node16 { keys, children } => {
                if n.num_children < 16 {
                    let m = n.num_children as usize;
                    let idx = keys.iter().position(|&key| c < key).unwrap_or(m);
                    //keys.copy_within(idx..m, idx+1);
                    for i in (idx..m).rev() {
                        keys[i] = keys[i - 1];
                        children[i] = children[i - 1].take();
                    }

                    keys[idx] = c;
                    children[idx] = child;
                    n.num_children += 1;
                } else {
                    const INIT: MyNode = None;
                    let mut children_new: [MyNode; 48] = [INIT; 48];
                    let mut keys_new: [u8; 256] = [0; 256];

                    for i in 0..16 {
                        keys_new[keys[i] as usize] = (i + 1) as u8;
                        children_new[i] = children[i].take();
                    }

                    self.inner = ArtNodeInternalInner::Node48 {
                        keys: keys_new,
                        children: children_new,
                    };
                    self.add_child(c, child);
                }
            }
            ArtNodeInternalInner::Node48 { keys, children } => {
                if n.num_children < 48 {
                    let pos = children.iter().position(|child| child.is_none()).unwrap();
                    children[pos] = child;
                    keys[c as usize] = (pos + 1) as u8; // TODO: double check this
                    n.num_children += 1;
                } else {
                    // TODO: consider optimising this
                    const INIT: MyNode = None;
                    let mut children_new: [MyNode; 256] = [INIT; 256];
                    for (i, &key) in keys.iter().enumerate() {
                        if key != 0 {
                            children_new[i] = children[(key - 1) as usize].take();
                        }
                    }

                    self.inner = ArtNodeInternalInner::Node256 {
                        children: children_new,
                    };
                    self.add_child(c, child);
                }
            }
            ArtNodeInternalInner::Node256 { children } => {
                n.num_children += 1;
                children[c as usize] = child;
            }
        }
    }

    fn minimum(&mut self) -> Option<&mut ArtNodeLeaf>{
        let n = &self.header;
        match &mut self.inner{
            ArtNodeInternalInner::Node4 { children, .. } => children[0].as_mut(),
            ArtNodeInternalInner::Node16 { children,.. } => children[0].as_mut(),
            ArtNodeInternalInner::Node48 { keys, children,.. } => {
                let idx = keys.iter().position(|&key| key != 0).unwrap_or(48);
                let idx = (keys[idx] - 1) as usize;
                children[idx].as_mut()
            },
            ArtNodeInternalInner::Node256 {  children,.. } => {
                let idx = children.iter().position(|child| child.is_some());
                match idx{
                    None => None,
                    Some(i) => children[i].as_mut(),
                }
            },
        }.and_then(|next|next.minimum())
    }

    fn maximum(&mut self) -> Option<&mut ArtNodeLeaf>{
        let n = &self.header;
        match &mut self.inner{
            ArtNodeInternalInner::Node4 {  children, .. } => children[(n.num_children-1) as usize].as_mut(),
            ArtNodeInternalInner::Node16 {children,.. } => children[(n.num_children-1) as usize].as_mut(),
            ArtNodeInternalInner::Node48 { keys, children,.. } => {
                let idx = keys.iter().rev().position(|&key| key != 0).unwrap_or(0);
                let idx = (keys[idx] - 1) as usize;
                children[idx].as_mut()
            },
            ArtNodeInternalInner::Node256 {  children,.. } => {
                let idx = children.iter().rev().position(|child| child.is_some());
                match idx{
                    None => None,
                    Some(i) => children[i].as_mut(),
                }
            },
        }.and_then(|next|next.maximum())
    }

    /*
    fn partial_len(&self) -> u32 {
        match self {
            ArtNode::Node4 { n, .. } => n.partial_len,
            ArtNode::Node16 { n, .. } => n.partial_len,
            ArtNode::Node48 { n, .. } => n.partial_len,
            ArtNode::Node256 { n, .. } => n.partial_len,
        }
    }

    fn partial(&self) -> [u8; MAX_PREFIX_LEN as usize] {
        match self {
            ArtNode::Node4 { n, .. } => n.partial,
            ArtNode::Node16 { n, .. } => n.partial,
            ArtNode::Node48 { n, .. } => n.partial,
            ArtNode::Node256 { n, .. } => n.partial,
        }
    }

     */
}

impl ArtNodeInternal{
    /// Calculates the index at which the prefixes mismatch
    fn prefix_mismatch(&mut self, key: &[u8], key_len: u32, depth: u32) -> u32{
        let max_cmp = min(min(MAX_PREFIX_LEN, self.header.partial_len), key_len - depth) as usize;
        let idx = (0..max_cmp).into_iter().position(|i| self.header.partial[i] != key[(depth as usize +i) as usize]).unwrap_or(MAX_PREFIX_LEN as usize);

        // If the prefix is short we can avoid finding a leaf
        if self.header.partial_len > MAX_PREFIX_LEN{
            // Prefix is longer than what we've checked, find a leaf
            let l = self.minimum().unwrap(); // TODO: check
            let max_cmp = (min(l.key_len, key_len) - depth) as usize;
            for i in idx..max_cmp{
                if l.key[(i+depth as usize)] != key[(depth as usize+i) as usize]{
                    return i as u32;
                }
            }
        }

        return idx as u32;
    }
}


impl InternalNodeHeader {
    /// Returns the number of prefix characters shared between
    /// the key and node.
    fn check_prefix(&self, key: &[u8], key_len: u32, depth: u32) -> u32{
        let max_cmp = min(min(self.partial_len, MAX_PREFIX_LEN), key_len -depth);
        for idx in 0..max_cmp{
            if self.partial[idx as usize] != key[(depth + idx) as usize]{
                return idx;
            }
        }
        return max_cmp;
    }
}

impl ArtNodeLeaf {

    fn new(key: Box<[u8]>, key_len: u32, value: u32) -> Self{
        let mut key_clone = Vec::with_capacity(key.len());
        key_clone.copy_from_slice(key.as_ref());
        Self{
            value,
            key_len,
            key: key_clone.into_boxed_slice(),
        }
    }

    /// Checks if a leaf matches
    /// @return 0 on success.
    fn leaf_matches(&self, key: &[u8], key_len: u32, _depth: u32) -> bool{
        if self.key_len != key_len {
            return false;
        }
        self.key == Box::from(key)
    }

    fn longest_common_prefix(&mut self, other: &mut Self, depth: u32) -> u32{
        let max_cmp = min(self.key_len, other.key_len) - depth;
        for idx in depth..max_cmp{
            if self.key[idx as usize] != other.key[idx as usize] {
                return idx;
            }
        }
        return max_cmp;
    }
}
