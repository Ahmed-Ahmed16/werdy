use werdy_core::{Fraction, Mark, Target, convert_mark_for_target};

#[test]
fn target_form_change_preserves_done_and_partial_values() -> Result<(), Box<dyn std::error::Error>>
{
    let target = Target::new(5, "pages")?;
    assert_eq!(
        convert_mark_for_target(Mark::Done, None, Some(&target)),
        Mark::Count(5)
    );
    assert_eq!(
        convert_mark_for_target(Mark::Count(5), Some(&target), None),
        Mark::Done
    );
    assert_eq!(
        convert_mark_for_target(Mark::Count(2), Some(&target), None),
        Mark::partial_without_target()
    );
    assert_eq!(
        convert_mark_for_target(Mark::partial_without_target(), None, Some(&target)),
        Mark::Count(3)
    );
    assert_eq!(
        convert_mark_for_target(Mark::Partial(Fraction::ZERO), None, Some(&target)),
        Mark::Missed
    );
    Ok(())
}
