The first full job stopped with317 passed tests and one model-test failure.
VIEWS_NATIVE_CHECKPOINT_NOT_CURRENT correctly refused a Session whose test replaced
its editor with a different runtime instance identity. The corrected fixture
retains Session.editor freshness.instance_id when constructing its fixture editor.
No native checkpoint guard or runtime persistence contract was weakened.
