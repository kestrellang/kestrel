Find an issue in @docs/contributing/compiler_issues.md.
Create a temporary test file that reproduces the issue.
Run the cli on the test file, getting the syntax tree, semantic tree, and if necessary, execution graph.
Figure out the root cause of the issue, but don't fix the issue yet
Report back on what the root cause is, and how you would fix it.
Create a regression test in kestrel-test-suite.
Update @docs/contributing/compiler_issues.md to note that it is fixed and where the regression test is, and a summary of the root cause and fix.