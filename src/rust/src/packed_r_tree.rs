//! Create and read a [packed Hilbert R-Tree](https://en.wikipedia.org/wiki/Hilbert_R-tree#Packed_Hilbert_R-trees)
//! to enable fast bounding box spatial filtering.

use crate::http_reader::BufferedHttpClient;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::mem::size_of;
use std::{cmp, f64, u64, usize};

#[derive(Clone, PartialEq, Debug)]
#[repr(C)]
/// R-Tree node
pub struct NodeItem {
    min_x: f64, // double
    min_y: f64, // double
    max_x: f64, // double
    max_y: f64, // double
    /// Byte offset in feature data section
    offset: u64, // uint64_t
}

impl NodeItem {
    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> NodeItem {
        NodeItem {
            min_x,
            min_y,
            max_x,
            max_y,
            offset: 0,
        }
    }

    pub fn create(offset: u64) -> NodeItem {
        NodeItem {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
            offset,
        }
    }

    pub fn width(&self) -> f64 {
        self.max_x - self.min_x
    }

    pub fn height(&self) -> f64 {
        self.max_y - self.min_y
    }

    pub fn sum(mut a: NodeItem, b: &NodeItem) -> NodeItem {
        a.expand(b);
        a
    }

    fn expand(&mut self, r: &NodeItem) {
        if r.min_x < self.min_x {
            self.min_x = r.min_x;
        }
        if r.min_y < self.min_y {
            self.min_y = r.min_y;
        }
        if r.max_x > self.max_x {
            self.max_x = r.max_x;
        }
        if r.max_y > self.max_y {
            self.max_y = r.max_y;
        }
    }

    pub fn intersects(&self, r: &NodeItem) -> bool {
        if self.max_x < r.min_x {
            return false;
        }
        if self.max_y < r.min_y {
            return false;
        }
        if self.min_x > r.max_x {
            return false;
        }
        if self.min_y > r.max_y {
            return false;
        }
        true
    }

    // std::vector<double> NodeItem::toVector()
    // {
    //     return std::vector<double> { min_x, min_y, max_x, max_y };
    // }
}

#[derive(Debug)]
/// Bbox filter search result
pub struct SearchResultItem {
    /// Byte offset in feature data section
    pub offset: usize,
    /// Feature number
    pub index: usize,
}

const HILBERT_MAX: u32 = (1 << 16) - 1;

// Based on public domain code at https://github.com/rawrunprotected/hilbert_curves
fn hilbert(x: u32, y: u32) -> u32 {
    let mut a = x ^ y;
    let mut b = 0xFFFF ^ a;
    let mut c = 0xFFFF ^ (x | y);
    let mut d = x & (y ^ 0xFFFF);

    let mut aa = a | (b >> 1);
    let mut bb = (a >> 1) ^ a;
    let mut cc = ((c >> 1) ^ (b & (d >> 1))) ^ c;
    let mut dd = ((a & (c >> 1)) ^ (d >> 1)) ^ d;

    a = aa;
    b = bb;
    c = cc;
    d = dd;
    aa = (a & (a >> 2)) ^ (b & (b >> 2));
    bb = (a & (b >> 2)) ^ (b & ((a ^ b) >> 2));
    cc ^= (a & (c >> 2)) ^ (b & (d >> 2));
    dd ^= (b & (c >> 2)) ^ ((a ^ b) & (d >> 2));

    a = aa;
    b = bb;
    c = cc;
    d = dd;
    aa = (a & (a >> 4)) ^ (b & (b >> 4));
    bb = (a & (b >> 4)) ^ (b & ((a ^ b) >> 4));
    cc ^= (a & (c >> 4)) ^ (b & (d >> 4));
    dd ^= (b & (c >> 4)) ^ ((a ^ b) & (d >> 4));

    a = aa;
    b = bb;
    c = cc;
    d = dd;
    cc ^= (a & (c >> 8)) ^ (b & (d >> 8));
    dd ^= (b & (c >> 8)) ^ ((a ^ b) & (d >> 8));

    a = cc ^ (cc >> 1);
    b = dd ^ (dd >> 1);

    let mut i0 = x ^ y;
    let mut i1 = b | (0xFFFF ^ (i0 | a));

    i0 = (i0 | (i0 << 8)) & 0x00FF00FF;
    i0 = (i0 | (i0 << 4)) & 0x0F0F0F0F;
    i0 = (i0 | (i0 << 2)) & 0x33333333;
    i0 = (i0 | (i0 << 1)) & 0x55555555;

    i1 = (i1 | (i1 << 8)) & 0x00FF00FF;
    i1 = (i1 | (i1 << 4)) & 0x0F0F0F0F;
    i1 = (i1 | (i1 << 2)) & 0x33333333;
    i1 = (i1 | (i1 << 1)) & 0x55555555;

    let value = (i1 << 1) | i0;

    value
}

fn hilbert_bbox(r: &NodeItem, hilbert_max: u32, extent: &NodeItem) -> u32 {
    // calculate bbox center and scale to hilbert_max
    let x = (hilbert_max as f64 * ((r.min_x + r.max_x) / 2.0 - extent.min_x) / extent.width())
        .floor() as u32;
    let y = (hilbert_max as f64 * ((r.min_y + r.max_y) / 2.0 - extent.min_y) / extent.height())
        .floor() as u32;
    hilbert(x, y)
}

pub fn hilbert_sort(items: &mut Vec<NodeItem>) {
    let extent = calc_extent(items);
    items.sort_by(|a, b| {
        let ha = hilbert_bbox(a, HILBERT_MAX, &extent);
        let hb = hilbert_bbox(b, HILBERT_MAX, &extent);
        hb.partial_cmp(&ha).unwrap() // ha > hb
    });
}

// void hilbert_sort_shared_ptr(std::vector<std::shared_ptr<Item>> &items)
// {
//     NodeItem extent = std::accumulate(items.begin(), items.end(), NodeItem::create(0), [] (NodeItem a, std::shared_ptr<Item> b) {
//         a.expand(b->node_item);
//         return a;
//     });
//     std::sort(items.begin(), items.end(), [&extent] (std::shared_ptr<Item> a, std::shared_ptr<Item> b) {
//         uint32_t ha = hilbert(a->node_item, hilbert_max, extent);
//         uint32_t hb = hilbert(b->node_item, hilbert_max, extent);
//         return ha > hb;
//     });
// }

pub fn calc_extent(nodes: &Vec<NodeItem>) -> NodeItem {
    nodes.iter().fold(NodeItem::create(0), |mut a, b| {
        a.expand(b);
        a
    })
}

// NodeItem calc_extent_shared_ptr(const std::vector<std::shared_ptr<Item>> &items)
// {
//     NodeItem extent = std::accumulate(items.begin(), items.end(), NodeItem::create(0), [] (NodeItem a, std::shared_ptr<Item> b) {
//         a.expand(b->node_item);
//         return a;
//     });
//     return extent;
// }

/// Packed Hilbert R-Tree
pub struct PackedRTree {
    extent: NodeItem,
    node_items: Vec<NodeItem>,
    num_items: usize,
    num_nodes: usize,
    node_size: u16,
    level_bounds: Vec<(usize, usize)>,
}

impl PackedRTree {
    pub const DEFAULT_NODE_SIZE: u16 = 16;

    fn init(&mut self, node_size: u16) {
        assert!(node_size >= 2, "Node size must be at least 2");
        assert!(self.num_items > 0, "Cannot create empty tree");
        self.node_size = cmp::min(cmp::max(node_size, 2u16), 65535u16);
        self.level_bounds = PackedRTree::generate_level_bounds(self.num_items, self.node_size);
        self.num_nodes = self.level_bounds.first().unwrap().1;
        self.node_items = vec![NodeItem::create(0); self.num_nodes];
    }

    fn generate_level_bounds(num_items: usize, node_size: u16) -> Vec<(usize, usize)> {
        assert!(node_size >= 2, "Node size must be at least 2");
        assert!(num_items > 0, "Cannot create empty tree");
        assert!(
            num_items <= usize::MAX - ((num_items / node_size as usize) * 2),
            "Number of items too large"
        );

        // number of nodes per level in bottom-up order
        let mut level_num_nodes: Vec<usize> = Vec::new();
        let mut n = num_items;
        let mut num_nodes = n;
        level_num_nodes.push(n);
        loop {
            n = (n + node_size as usize - 1) / node_size as usize;
            num_nodes += n;
            level_num_nodes.push(n);
            if n == 1 {
                break;
            }
        }
        // bounds per level in reversed storage order (top-down)
        let mut level_offsets: Vec<usize> = Vec::new();
        n = num_nodes;
        for size in &level_num_nodes {
            level_offsets.push(n - size);
            n -= size;
        }
        level_offsets.reverse();
        level_num_nodes.reverse();
        let mut level_bounds = Vec::new();
        for i in 0..level_num_nodes.len() {
            level_bounds.push((level_offsets[i], level_offsets[i] + level_num_nodes[i]));
        }
        level_bounds.reverse();
        level_bounds
    }

    fn generate_nodes(&mut self) {
        for i in 0..self.level_bounds.len() - 1 {
            let mut pos = self.level_bounds[i].0;
            let end = self.level_bounds[i].1;
            let mut newpos = self.level_bounds[i + 1].0;
            while pos < end {
                let mut node = NodeItem::create(pos as u64);
                for _j in 0..self.node_size {
                    if pos >= end {
                        break;
                    }
                    node.expand(&self.node_items[pos]);
                    pos += 1;
                }
                self.node_items[newpos] = node;
                newpos += 1;
            }
        }
    }

    fn read_data(&mut self, data: &mut dyn Read) {
        let mut n = NodeItem::create(0);
        let buf: &mut [u8] = unsafe {
            std::slice::from_raw_parts_mut(&mut n as *mut _ as *mut u8, size_of::<NodeItem>())
        };
        for i in 0..self.num_nodes {
            data.read_exact(buf).unwrap();
            self.node_items[i] = n.clone();
            self.extent.expand(&n);
        }
    }

    async fn read_http(&mut self, client: &mut BufferedHttpClient<'_>, index_begin: usize) {
        let min_req_size = self.size();
        let mut pos = index_begin;
        for i in 0..self.num_nodes {
            let bytes = client
                .get(pos, size_of::<NodeItem>(), min_req_size)
                .await
                .unwrap();
            let p = bytes.as_ptr() as *const [u8; size_of::<NodeItem>()];
            let n: NodeItem = unsafe { std::mem::transmute(*p) };
            self.node_items[i] = n.clone();
            self.extent.expand(&n);
            pos += size_of::<NodeItem>();
        }
    }

    pub fn build(nodes: &Vec<NodeItem>, extent: &NodeItem, node_size: u16) -> PackedRTree {
        let mut tree = PackedRTree {
            extent: extent.clone(),
            node_items: Vec::new(),
            num_items: nodes.len(),
            num_nodes: 0,
            node_size: 0,
            level_bounds: Vec::new(),
        };
        tree.init(node_size);
        for i in 0..tree.num_items {
            tree.node_items[(tree.num_nodes - tree.num_items + i)] = nodes[i].clone();
        }
        tree.generate_nodes();
        tree
    }

    pub fn from_buf(data: &mut dyn Read, num_items: usize, node_size: u16) -> PackedRTree {
        let mut tree = PackedRTree {
            extent: NodeItem::create(0),
            node_items: Vec::new(),
            num_items: num_items,
            num_nodes: 0,
            node_size: 0,
            level_bounds: Vec::new(),
        };
        tree.init(node_size);
        tree.read_data(data);
        tree
    }

    pub async fn from_http(
        client: &mut BufferedHttpClient<'_>,
        index_begin: usize,
        num_items: usize,
        node_size: u16,
    ) -> PackedRTree {
        let mut tree = PackedRTree {
            extent: NodeItem::create(0),
            node_items: Vec::new(),
            num_items: num_items,
            num_nodes: 0,
            node_size: 0,
            level_bounds: Vec::new(),
        };
        tree.init(node_size);
        tree.read_http(client, index_begin).await;
        tree
    }

    pub fn search(&self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Vec<SearchResultItem> {
        let leaf_nodes_offset = self.level_bounds.first().unwrap().0;
        let n = NodeItem::new(min_x, min_y, max_x, max_y);
        let mut results = Vec::new();
        let mut queue = HashMap::new(); // C++: std::unordered_map
        queue.insert(0, self.level_bounds.len() - 1);
        while queue.len() != 0 {
            let next = queue.iter().next().unwrap();
            let node_index = *next.0;
            let level = *next.1;
            queue.remove(&node_index);
            let is_leaf_node = node_index >= self.num_nodes - self.num_items;
            // find the end index of the node
            let end = cmp::min(
                node_index + self.node_size as usize,
                self.level_bounds[level].1,
            );
            // search through child nodes
            for pos in node_index..end {
                let node_item = &self.node_items[pos];
                if !n.intersects(&node_item) {
                    continue;
                }
                if is_leaf_node {
                    results.push(SearchResultItem {
                        offset: node_item.offset as usize,
                        index: pos - leaf_nodes_offset,
                    });
                } else {
                    queue.insert(node_item.offset as usize, level - 1);
                }
            }
        }
        results
    }

    // std::vector<SearchResultItem> PackedRTree::streamSearch(
    //     const uint64_t num_items, const uint16_t node_size, const NodeItem& item,
    //     const std::function<void(uint8_t *, size_t, size_t)> &readNode)
    // {
    //     auto level_bounds = generate_level_bounds(num_items, node_size);
    //     uint64_t leaf_nodes_offset = level_bounds.front().first;
    //     uint64_t num_nodes = level_bounds.front().second;
    //     std::vector<NodeItem> node_items;
    //     node_items.reserve(node_size);
    //     uint8_t *nodesBuf = reinterpret_cast<uint8_t *>(node_items.data());
    //     // use ordered search queue to make index traversal in sequential order
    //     std::map<uint64_t, uint64_t> queue;
    //     std::vector<SearchResultItem> results;
    //     queue.insert(std::pair<uint64_t, uint64_t>(0, level_bounds.size() - 1));
    //     while(queue.size() != 0) {
    //         auto next = queue.begin();
    //         uint64_t node_index = next->first;
    //         uint64_t level = next->second;
    //         queue.erase(next);
    //         bool is_leaf_node = node_index >= num_nodes - num_items;
    //         // find the end index of the node
    //         uint64_t end = std::min(static_cast<uint64_t>(node_index + node_size), level_bounds[static_cast<size_t>(level)].second);
    //         uint64_t length = end - node_index;
    //         readNode(nodesBuf, static_cast<size_t>(node_index * sizeof(NodeItem)), static_cast<size_t>(length * sizeof(NodeItem)));
    //         // search through child nodes
    //         for (uint64_t pos = node_index; pos < end; pos++) {
    //             uint64_t nodePos = pos - node_index;
    //             auto node_item = node_items[static_cast<size_t>(nodePos)];
    //             if (!item.intersects(node_item))
    //                 continue;
    //             if (is_leaf_node)
    //                 results.push_back({ node_item.offset, pos - leaf_nodes_offset });
    //             else
    //                 queue.insert(std::pair<uint64_t, uint64_t>(node_item.offset, level - 1));
    //         }
    //     }
    //     return results;
    // }

    pub fn size(&self) -> usize {
        self.num_nodes * size_of::<NodeItem>()
    }

    pub fn index_size(num_items: usize, node_size: u16) -> usize {
        assert!(node_size >= 2, "Node size must be at least 2");
        assert!(num_items > 0, "Cannot create empty tree");
        let node_size_min = cmp::min(cmp::max(node_size, 2), 65535) as usize;
        // limit so that resulting size in bytes can be represented by uint64_t
        assert!(
            num_items <= 1 << 56,
            "Number of items must be less than 2^56"
        );
        let mut n = num_items;
        let mut num_nodes = n;
        loop {
            n = (n + node_size_min - 1) / node_size_min;
            num_nodes += n;
            if n == 1 {
                break;
            }
        }
        num_nodes * size_of::<NodeItem>()
    }

    pub fn stream_write(&self, out: &mut dyn Write) -> std::io::Result<()> {
        let buf: &[u8] = unsafe {
            std::slice::from_raw_parts(
                self.node_items.as_ptr() as *const u8,
                self.node_items.len() * size_of::<NodeItem>(),
            )
        };
        out.write_all(buf)
    }

    pub fn extent(&self) -> NodeItem {
        self.extent.clone()
    }
}

#[test]
fn tree_2items() {
    let mut nodes = Vec::new();
    nodes.push(NodeItem::new(0.0, 0.0, 1.0, 1.0));
    nodes.push(NodeItem::new(2.0, 2.0, 3.0, 3.0));
    let extent = calc_extent(&nodes);
    assert_eq!(extent, NodeItem::new(0.0, 0.0, 3.0, 3.0));
    assert!(nodes[0].intersects(&NodeItem::new(0.0, 0.0, 1.0, 1.0)));
    assert!(nodes[1].intersects(&NodeItem::new(2.0, 2.0, 3.0, 3.0)));
    hilbert_sort(&mut nodes);
    let mut offset = 0;
    for mut node in &mut nodes {
        node.offset = offset as u64;
        offset += size_of::<NodeItem>();
    }
    assert!(nodes[1].intersects(&NodeItem::new(0.0, 0.0, 1.0, 1.0)));
    assert!(nodes[0].intersects(&NodeItem::new(2.0, 2.0, 3.0, 3.0)));
    let tree = PackedRTree::build(&nodes, &extent, PackedRTree::DEFAULT_NODE_SIZE);
    let list = tree.search(0.0, 0.0, 1.0, 1.0);
    assert_eq!(list.len(), 1);
    assert!(nodes[list[0].index].intersects(&NodeItem::new(0.0, 0.0, 1.0, 1.0)));
}

#[test]
fn tree_19items_roundtrip_stream_search() {
    let mut nodes = Vec::new();
    nodes.push(NodeItem::new(0.0, 0.0, 1.0, 1.0));
    nodes.push(NodeItem::new(2.0, 2.0, 3.0, 3.0));
    nodes.push(NodeItem::new(100.0, 100.0, 110.0, 110.0));
    nodes.push(NodeItem::new(101.0, 101.0, 111.0, 111.0));
    nodes.push(NodeItem::new(102.0, 102.0, 112.0, 112.0));
    nodes.push(NodeItem::new(103.0, 103.0, 113.0, 113.0));
    nodes.push(NodeItem::new(104.0, 104.0, 114.0, 114.0));
    nodes.push(NodeItem::new(10010.0, 10010.0, 10110.0, 10110.0));
    nodes.push(NodeItem::new(10010.0, 10010.0, 10110.0, 10110.0));
    nodes.push(NodeItem::new(10010.0, 10010.0, 10110.0, 10110.0));
    nodes.push(NodeItem::new(10010.0, 10010.0, 10110.0, 10110.0));
    nodes.push(NodeItem::new(10010.0, 10010.0, 10110.0, 10110.0));
    nodes.push(NodeItem::new(10010.0, 10010.0, 10110.0, 10110.0));
    nodes.push(NodeItem::new(10010.0, 10010.0, 10110.0, 10110.0));
    nodes.push(NodeItem::new(10010.0, 10010.0, 10110.0, 10110.0));
    nodes.push(NodeItem::new(10010.0, 10010.0, 10110.0, 10110.0));
    nodes.push(NodeItem::new(10010.0, 10010.0, 10110.0, 10110.0));
    nodes.push(NodeItem::new(10010.0, 10010.0, 10110.0, 10110.0));
    nodes.push(NodeItem::new(10010.0, 10010.0, 10110.0, 10110.0));
    let extent = calc_extent(&nodes);
    hilbert_sort(&mut nodes);
    let mut offset = 0;
    for mut node in &mut nodes {
        node.offset = offset as u64;
        offset += size_of::<NodeItem>();
    }
    let tree = PackedRTree::build(&nodes, &extent, PackedRTree::DEFAULT_NODE_SIZE);
    let list = tree.search(102.0, 102.0, 103.0, 103.0);
    assert_eq!(list.len(), 4);
    for i in 0..list.len() {
        assert!(nodes[list[i].index].intersects(&NodeItem::new(102.0, 102.0, 103.0, 103.0)));
    }
    let mut tree_data: Vec<u8> = Vec::new();
    let res = tree.stream_write(&mut tree_data);
    assert!(res.is_ok());
    assert_eq!(tree_data.len(), (nodes.len() + 3) * 40);
    assert_eq!(size_of::<NodeItem>(), 40);

    let tree2 = PackedRTree::from_buf(
        &mut &tree_data[..],
        nodes.len(),
        PackedRTree::DEFAULT_NODE_SIZE,
    );
    let list = tree2.search(102.0, 102.0, 103.0, 103.0);
    assert_eq!(list.len(), 4);
    for i in 0..list.len() {
        assert!(nodes[list[i].index].intersects(&NodeItem::new(102.0, 102.0, 103.0, 103.0)));
    }

    // auto readNode = [data] (uint8_t *buf, uint32_t i, uint32_t s) {
    //     //std::cout << "i: " << i << std::endl;
    //     std::copy(data + i, data + i + s, buf);
    // };
    // auto list3 = PackedRTree::streamSearch(nodes.size(), 16, {102, 102, 103, 103}, readNode);
    // REQUIRE(list3.size() == 4);
    // for (uint32_t i = 0; i < list3.size(); i++) {
    //     REQUIRE(nodes[list3[i].index].intersects({102, 102, 103, 103}) == true);
    // }
}

#[test]
fn tree_100_000_items_in_denmark() {
    use rand::distributions::{Distribution, Uniform};

    let unifx = Uniform::from(466379..708929);
    let unify = Uniform::from(6096801..6322352);
    let mut rng = rand::thread_rng();

    let mut nodes = Vec::new();
    for _ in 0..100000 {
        let x = unifx.sample(&mut rng) as f64;
        let y = unify.sample(&mut rng) as f64;
        nodes.push(NodeItem::new(x, y, x, y));
    }

    let extent = calc_extent(&nodes);
    hilbert_sort(&mut nodes);
    let tree = PackedRTree::build(&nodes, &extent, PackedRTree::DEFAULT_NODE_SIZE);
    let list = tree.search(690407.0, 6063692.0, 811682.0, 6176467.0);

    for i in 0..list.len() {
        assert!(nodes[list[i].index]
            .intersects(&NodeItem::new(690407.0, 6063692.0, 811682.0, 6176467.0)));
    }

    let mut tree_data: Vec<u8> = Vec::new();
    let res = tree.stream_write(&mut tree_data);
    assert!(res.is_ok());

    // auto list2 = PackedRTree::streamSearch(nodes.size(), 16, {690407, 6063692, 811682, 6176467}, readNode);
    // for (uint64_t i = 0; i < list2.size(); i++)
    //     CHECK(nodes[list2[i].index].intersects({690407, 6063692, 811682, 6176467}) == true);
}
