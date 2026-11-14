// Where the program points to: its repository, its releases and its website. One place, so that a
// move changes one line.
export const REPOSITORY = "https://github.com/Leunami52/erdtex";
export const RELEASES = `${REPOSITORY}/releases/latest`;
export const WEBSITE = "https://leunami52.github.io/erdtex/";

import imprintText from "../../../texts/imprint.md?raw";
import privacyText from "../../../texts/privacy.md?raw";

/** The imprint, or "" while its file holds nothing but its comment (then Help shows no entry). */
export const IMPRINT = imprintText.replace(/<!--[\s\S]*?-->/g, "").trim();
/** What the website stores, and what not. */
export const PRIVACY = privacyText.trim();
