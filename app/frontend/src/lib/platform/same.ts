// Both platforms offer the same functions with the same types: a type error here names what one
// of them lacks or has differently. (Nothing in it runs.)
import type * as desktop from "./desktop";
import type * as web from "./web";

type Shape<T> = { [K in keyof T]: T[K] };
// what only the browser offers, for its tests and for the menus
type WebOnly = "addFile" | "writtenText";

export type DesktopHasAll = Shape<Omit<typeof web, WebOnly>> extends Shape<typeof desktop> ? true : never;
export type WebHasAll = Shape<typeof desktop> extends Shape<Omit<typeof web, WebOnly>> ? true : never;
export const checked: [DesktopHasAll, WebHasAll] = [true, true];
