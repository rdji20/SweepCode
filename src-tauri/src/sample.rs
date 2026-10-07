//! A built-in problem so the app is usable offline and on first launch.
//! The description is our own short wording, not LeetCode's text.

use crate::model::{Meta, Param, Problem, TestCase};

pub fn two_sum() -> Problem {
    let content = r#"<p>You get an array of integers <code>nums</code> and an integer <code>target</code>.
Return the indices of the two different elements whose sum equals <code>target</code>.</p>
<p>Exactly one pair works for every input, and you may return the indices in any order.</p>
<p><strong>Example 1</strong></p>
<pre><strong>Input:</strong> nums = [2,7,11,15], target = 9
<strong>Output:</strong> [0,1]
</pre>
<p><strong>Example 2</strong></p>
<pre><strong>Input:</strong> nums = [3,2,4], target = 6
<strong>Output:</strong> [1,2]
</pre>
<p><strong>Example 3</strong></p>
<pre><strong>Input:</strong> nums = [3,3], target = 6
<strong>Output:</strong> [0,1]
</pre>
<p><em>This is the offline sample. Search for any problem in the top bar to load it from leetcode.com.</em></p>"#;
    let ex = |a: &str, b: &str, out: &str| TestCase { inputs: vec![a.into(), b.into()], expected: Some(out.into()) };
    Problem {
        id: "1".into(),
        title: "Two Sum".into(),
        slug: "two-sum".into(),
        difficulty: "Easy".into(),
        paid_only: false,
        content: content.into(),
        java_code: Some("class Solution {\n    public int[] twoSum(int[] nums, int target) {\n        \n    }\n}".into()),
        meta: Some(Meta::Function {
            method: "twoSum".into(),
            params: vec![
                Param { name: "nums".into(), ty: "integer[]".into() },
                Param { name: "target".into(), ty: "integer".into() },
            ],
            return_type: "integer[]".into(),
            output_param: None,
        }),
        examples: vec![ex("[2,7,11,15]", "9", "[0,1]"), ex("[3,2,4]", "6", "[1,2]"), ex("[3,3]", "6", "[0,1]")],
        tags: vec!["Array".into(), "Hash Table".into()],
        hints: vec![],
        any_order: true,
        source: "sample".into(),
    }
}
