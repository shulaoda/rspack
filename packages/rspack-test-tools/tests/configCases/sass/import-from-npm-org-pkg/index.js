const fs = __non_webpack_require__("fs");
const path = __non_webpack_require__("path");

it('should work when "@import" at-rules from scoped npm packages', () => {
	require("./index.scss");
	const css = fs.readFileSync(path.resolve(__dirname, "bundle0.css"), "utf-8");
	expect(css.includes(".org-pkg") && css.includes(".scoped-npm-pkg-foo")).toBe(
		true
	);
});
