//! The flow-contract label tests, in their own file so the contracts stay inside the
//! line ceiling.
//! 流契约标签的测试，放在独立文件里，使契约本体留在行数上限之内。

use super::{ContractId, FlowContract, OwnedFlowContract};

/// An output label the table does not know must not be smuggled past the
/// gate by comparing two unknowns with each other.
/// 语义表不认识的输出标签，不得靠"两个未知标签互相比较"混过关口。
#[test]
fn unknown_output_labels_must_agree_literally() {
    let target = FlowContract::new(
        ContractId::new("render.v1"),
        1,
        "LocalCoordinates",
        "CanvasFrame",
    );
    let different = FlowContract::new(
        ContractId::new("render.v1"),
        1,
        "LocalCoordinates",
        "TotallyDifferentType",
    );
    assert!(!target.semantically_compatible_with(different));
    assert!(target.semantically_compatible_with(target));
}

/// Spelling differences are still authorised where the table knows the
/// domain, as long as the unknown side agrees literally.
/// 语义表认识的域仍允许拼写差异，只要未知的那一侧字面一致。
#[test]
fn known_domains_keep_accepting_spelling_differences() {
    let target = FlowContract::new(
        ContractId::new("render.v1"),
        1,
        "LocalCoordinates",
        "CanvasFrame",
    );
    let respelled = FlowContract::new(
        ContractId::new("render.v1"),
        1,
        "local_coordinates",
        "CanvasFrame",
    );
    assert!(target.semantically_compatible_with(respelled));
    let respelled_with_other_output = FlowContract::new(
        ContractId::new("render.v1"),
        1,
        "local_coordinates",
        "TotallyDifferentType",
    );
    assert!(!target.semantically_compatible_with(respelled_with_other_output));
}

/// The compiled and owned twins share one field comparison and one
/// semantic-label core, so each pair must get the same answer across the
/// matrix: the literal pair (`compatible_with` ↔ owned `==`) and the semantic
/// pair (both `semantically_compatible_with`).
/// 编译期与 owned 孪生共用一套字段比较与一个语义标签核，因此每一对在矩阵上都必须
/// 得到相同答案：字面对（`compatible_with` ↔ owned `==`）与语义对（两个
/// `semantically_compatible_with`）。
#[test]
fn static_and_owned_flow_contracts_compare_identically() {
    let cases = [
        ("render.v1", 1, "LocalCoordinates", "CanvasFrame"),
        ("render.v1", 1, "local_coordinates", "CanvasFrame"),
        ("render.v1", 1, "LocalCoordinates", "TotallyDifferentType"),
        ("render.v1", 2, "LocalCoordinates", "CanvasFrame"),
        ("render.v2", 1, "LocalCoordinates", "CanvasFrame"),
        ("", 0, "", ""),
    ];
    for &(id, version, input, output) in &cases {
        for &(other_id, other_version, other_input, other_output) in &cases {
            let left = FlowContract::new(ContractId::new(id), version, input, output);
            let right = FlowContract::new(
                ContractId::new(other_id),
                other_version,
                other_input,
                other_output,
            );
            let owned_left = OwnedFlowContract::from(left);
            let owned_right = OwnedFlowContract::from(right);
            assert_eq!(
                left.is_declared(),
                owned_left.is_declared(),
                "is_declared disagrees for {left:?}"
            );
            assert_eq!(
                left.compatible_with(right),
                owned_left == owned_right,
                "literal comparison disagrees for {left:?} vs {right:?}"
            );
            assert_eq!(
                left.semantically_compatible_with(right),
                owned_left.semantically_compatible_with(&owned_right),
                "semantic comparison disagrees for {left:?} vs {right:?}"
            );
        }
    }
}

/// F3 pin: `compatible_with` is literal-only, and the semantic entry point
/// asks a different question. `LocalCoordinates` versus `local_coordinates`
/// is the counterexample — the literal pair rejects the pair, the semantic
/// pair accepts it. The matrix above cannot expose this divergence because
/// it pairs each entry point with its true counterpart, so this test states
/// the divergence and both pairings explicitly.
/// F3 钉：`compatible_with` 只做字面比较，语义入口问的是另一个问题。
/// `LocalCoordinates` 与 `local_coordinates` 就是反例——字面对拒绝这一对，语义对
/// 接受。上面的矩阵无法暴露这处分歧，因为它把每个入口与它真正的对应物配对，因此本
/// 测试把这处分歧与两种配对都显式写出。
#[test]
fn compatible_with_stays_literal_while_the_semantic_entry_point_folds_spellings() {
    let literal = FlowContract::new(
        ContractId::new("render.v1"),
        1,
        "LocalCoordinates",
        "CanvasFrame",
    );
    let respelled = FlowContract::new(
        ContractId::new("render.v1"),
        1,
        "local_coordinates",
        "CanvasFrame",
    );
    assert!(
        !literal.compatible_with(respelled),
        "compatible_with must stay a literal comparison"
    );
    assert!(
        literal.semantically_compatible_with(respelled),
        "the semantic entry point folds known-domain spellings"
    );
    assert_eq!(
        literal.compatible_with(respelled),
        OwnedFlowContract::from(literal) == OwnedFlowContract::from(respelled)
    );
    assert_eq!(
        literal.semantically_compatible_with(respelled),
        OwnedFlowContract::from(literal)
            .semantically_compatible_with(&OwnedFlowContract::from(respelled))
    );
}
