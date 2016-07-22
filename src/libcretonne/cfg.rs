//! A control flow graph represented as mappings of extended basic blocks to their predecessors
//! and successors. Successors are represented as extended basic blocks while predecessors are
//! represented by basic blocks.
//! BasicBlocks are denoted by tuples of EBB and branch/jump instructions. Each predecessor
//! tuple corresponds to the end of a basic block.
//!
//! ```c
//!     Ebb0:
//!         ...          ; beginning of basic block
//!
//!         ...
//!
//!         brz vx, Ebb1 ; end of basic block
//!
//!         ...          ; beginning of basic block
//!
//!         ...
//!
//!         jmp Ebb2     ; end of basic block
//! ```
//!
//! Here Ebb1 and Ebb2 would each have a single predecessor denoted as (Ebb0, `brz vx, Ebb1`)
//! and (Ebb0, `jmp Ebb2`) respectively.

use repr::Function;
use repr::entities::{Inst, Ebb};
use repr::instructions::InstructionData;
use entity_map::EntityMap;

/// A basic block denoted by its enclosing Ebb and last instruction.
pub type BasicBlock = (Ebb, Inst);

/// A container for the successors and predecessors of some Ebb.
#[derive(Debug)]
pub struct CFGNode {
    pub successors: Vec<Ebb>,
    pub predecessors: Vec<BasicBlock>,
}

impl CFGNode {
    pub fn new() -> CFGNode {
        CFGNode {
            successors: Vec::new(),
            predecessors: Vec::new(),
        }
    }
}

/// The Control Flow Graph maintains a mapping of ebbs to their predecessors
/// and successors where predecessors are basic blocks and successors are
/// extended basic blocks.
#[derive(Debug)]
pub struct ControlFlowGraph {
    data: EntityMap<Ebb, CFGNode>,
}

impl ControlFlowGraph {
    /// During initialization mappings will be generated for any existing
    /// blocks within the CFG's associated function.
    pub fn new(func: &Function) -> ControlFlowGraph {
        let mut cfg = ControlFlowGraph { data: EntityMap::new() };

        // Even ebbs without predecessors should show up in the CFG, albeit
        // with no entires.
        for _ in &func.layout {
            cfg.push_ebb();
        }

        for ebb in &func.layout {
            for inst in func.layout.ebb_insts(ebb) {
                match func.dfg[inst] {
                    InstructionData::Branch { ty: _, opcode: _, ref data } => {
                        cfg.add_successor(ebb, data.destination);
                        cfg.add_predecessor(data.destination, (ebb, inst));
                    }
                    InstructionData::Jump { ty: _, opcode: _, ref data } => {
                        cfg.add_successor(ebb, data.destination);
                        cfg.add_predecessor(data.destination, (ebb, inst));
                    }
                    _ => (),
                }
            }
        }
        cfg
    }

    pub fn push_ebb(&mut self) {
        self.data.push(CFGNode::new());
    }

    pub fn add_successor(&mut self, from: Ebb, to: Ebb) {
        self.data[from].successors.push(to);
    }

    pub fn add_predecessor(&mut self, ebb: Ebb, predecessor: BasicBlock) {
        self.data[ebb].predecessors.push(predecessor);
    }

    pub fn get_predecessors(&self, ebb: Ebb) -> &Vec<BasicBlock> {
        &self.data[ebb].predecessors
    }

    pub fn get_successors(&self, ebb: Ebb) -> &Vec<Ebb> {
        &self.data[ebb].successors
    }

    pub fn postorder_ebbs(&self) -> Vec<Ebb> {
        if self.len() < 1 {
            return Vec::new();
        }
        let mut stack_a = vec![Ebb::with_number(0).unwrap()];
        let mut stack_b = Vec::new();
        while stack_a.len() > 0 {
            let cur = stack_a.pop().unwrap();
            for child in &self.data[cur].successors {
                if *child != cur && !stack_a.contains(child) {
                    stack_a.push(child.clone());
                }
            }
            stack_b.push(cur);
        }
        stack_b
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn predecessors_iter(&self) -> CFGPredecessorsIter {
        CFGPredecessorsIter {
            cur: 0,
            cfg: &self,
        }
    }
}

/// Iterate through every mapping of ebb to predecessors in the CFG
pub struct CFGPredecessorsIter<'a> {
    cfg: &'a ControlFlowGraph,
    cur: usize,
}

impl<'a> Iterator for CFGPredecessorsIter<'a> {
    type Item = (Ebb, &'a Vec<BasicBlock>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur < self.cfg.len() {
            let ebb = Ebb::with_number(self.cur as u32).unwrap();
            let bbs = self.cfg.get_predecessors(ebb);
            self.cur += 1;
            Some((ebb, bbs))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use repr::Function;

    use test_utils::make_inst;

    #[test]
    fn empty() {
        let func = Function::new();
        let cfg = ControlFlowGraph::new(&func);
        assert_eq!(None, cfg.predecessors_iter().next());
    }

    #[test]
    fn no_predecessors() {
        let mut func = Function::new();
        let ebb0 = func.dfg.make_ebb();
        let ebb1 = func.dfg.make_ebb();
        let ebb2 = func.dfg.make_ebb();
        func.layout.append_ebb(ebb0);
        func.layout.append_ebb(ebb1);
        func.layout.append_ebb(ebb2);

        let cfg = ControlFlowGraph::new(&func);
        let nodes = cfg.predecessors_iter().collect::<Vec<_>>();
        assert_eq!(nodes.len(), 3);

        let mut fun_ebbs = func.layout.ebbs();
        for (ebb, predecessors) in nodes {
            assert_eq!(ebb, fun_ebbs.next().unwrap());
            assert_eq!(predecessors.len(), 0);
            assert_eq!(predecessors.len(), 0);
            assert_eq!(cfg.get_successors(ebb).len(), 0);
        }
    }

    #[test]
    fn branches_and_jumps() {
        let mut func = Function::new();
        let ebb0 = func.dfg.make_ebb();
        let ebb1 = func.dfg.make_ebb();
        let ebb2 = func.dfg.make_ebb();
        func.layout.append_ebb(ebb0);
        func.layout.append_ebb(ebb1);
        func.layout.append_ebb(ebb2);

        let br_ebb0_ebb2 = make_inst::branch(&mut func, ebb2);
        func.layout.append_inst(br_ebb0_ebb2, ebb0);

        let jmp_ebb0_ebb1 = make_inst::jump(&mut func, ebb1);
        func.layout.append_inst(jmp_ebb0_ebb1, ebb0);

        let br_ebb1_ebb1 = make_inst::branch(&mut func, ebb1);
        func.layout.append_inst(br_ebb1_ebb1, ebb1);

        let jmp_ebb1_ebb2 = make_inst::jump(&mut func, ebb2);
        func.layout.append_inst(jmp_ebb1_ebb2, ebb1);

        let cfg = ControlFlowGraph::new(&func);

        let ebb0_predecessors = cfg.get_predecessors(ebb0);
        let ebb1_predecessors = cfg.get_predecessors(ebb1);
        let ebb2_predecessors = cfg.get_predecessors(ebb2);

        let ebb0_successors = cfg.get_successors(ebb0);
        let ebb1_successors = cfg.get_successors(ebb1);
        let ebb2_successors = cfg.get_successors(ebb2);

        assert_eq!(ebb0_predecessors.len(), 0);
        assert_eq!(ebb1_predecessors.len(), 2);
        assert_eq!(ebb2_predecessors.len(), 2);

        assert_eq!(ebb1_predecessors.contains(&(ebb0, jmp_ebb0_ebb1)), true);
        assert_eq!(ebb1_predecessors.contains(&(ebb1, br_ebb1_ebb1)), true);
        assert_eq!(ebb2_predecessors.contains(&(ebb0, br_ebb0_ebb2)), true);
        assert_eq!(ebb2_predecessors.contains(&(ebb1, jmp_ebb1_ebb2)), true);

        assert_eq!(ebb0_successors.len(), 2);
        assert_eq!(ebb1_successors.len(), 2);
        assert_eq!(ebb2_successors.len(), 0);

        assert_eq!(ebb0_successors.contains(&ebb1), true);
        assert_eq!(ebb0_successors.contains(&ebb2), true);
        assert_eq!(ebb1_successors.contains(&ebb1), true);
        assert_eq!(ebb1_successors.contains(&ebb2), true);
    }

    #[test]
    fn postorder_traversal() {
        let mut func = Function::new();
        let ebb0 = func.dfg.make_ebb();
        let ebb1 = func.dfg.make_ebb();
        let ebb2 = func.dfg.make_ebb();
        let ebb3 = func.dfg.make_ebb();
        let ebb4 = func.dfg.make_ebb();
        let ebb5 = func.dfg.make_ebb();

        func.layout.append_ebb(ebb0);
        func.layout.append_ebb(ebb1);
        func.layout.append_ebb(ebb2);
        func.layout.append_ebb(ebb3);
        func.layout.append_ebb(ebb4);
        func.layout.append_ebb(ebb5);

        let br_ebb0_ebb1 = make_inst::branch(&mut func, ebb1);
        func.layout.append_inst(br_ebb0_ebb1, ebb0);

        let jmp_ebb0_ebb2 = make_inst::jump(&mut func, ebb2);
        func.layout.append_inst(jmp_ebb0_ebb2, ebb0);

        let br_ebb2_ebb2 = make_inst::branch(&mut func, ebb2);
        func.layout.append_inst(br_ebb2_ebb2, ebb2);

        let br_ebb2_ebb1 = make_inst::branch(&mut func, ebb1);
        func.layout.append_inst(br_ebb2_ebb1, ebb2);

        let jmp_ebb1_ebb3 = make_inst::jump(&mut func, ebb3);
        func.layout.append_inst(jmp_ebb1_ebb3, ebb1);

        let br_ebb2_ebb4 = make_inst::branch(&mut func, ebb4);
        func.layout.append_inst(br_ebb2_ebb4, ebb2);

        let jmp_ebb2_ebb5 = make_inst::jump(&mut func, ebb5);
        func.layout.append_inst(jmp_ebb2_ebb5, ebb2);

        let cfg = ControlFlowGraph::new(&func);
        assert_eq!(cfg.postorder_ebbs(),
                   vec![ebb0, ebb2, ebb5, ebb4, ebb1, ebb3]);
    }

    #[test]
    fn loop_edge() {
        let mut func = Function::new();
        let ebb0 = func.dfg.make_ebb();
        let ebb1 = func.dfg.make_ebb();
        let ebb2 = func.dfg.make_ebb();
        let ebb3 = func.dfg.make_ebb();
        func.layout.append_ebb(ebb0);
        func.layout.append_ebb(ebb1);
        func.layout.append_ebb(ebb2);
        func.layout.append_ebb(ebb3);

        let jmp_ebb0_ebb1 = make_inst::jump(&mut func, ebb1);
        let br_ebb1_ebb3 = make_inst::branch(&mut func, ebb3);
        let jmp_ebb1_ebb2 = make_inst::jump(&mut func, ebb2);
        let jmp_ebb2_ebb3 = make_inst::jump(&mut func, ebb3);

        func.layout.append_inst(jmp_ebb0_ebb1, ebb0);
        func.layout.append_inst(br_ebb1_ebb3, ebb1);
        func.layout.append_inst(jmp_ebb1_ebb2, ebb1);
        func.layout.append_inst(jmp_ebb2_ebb3, ebb2);

        let cfg = ControlFlowGraph::new(&func);
        assert_eq!(cfg.postorder_ebbs(), vec![ebb0, ebb1, ebb2, ebb3]);
    }
}
