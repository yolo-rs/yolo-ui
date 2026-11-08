// SPDX-License-Identifier: Apache-2.0
// next/font/google shim: one factory per family (the 92 most used) plus createGoogleFont() for any other family.

import { googleFont, type PlannedFont } from "../fonts.ts";
import type { GoogleFontOptions } from "../types.ts";

/** Factory signature of a Google font family (`Inter({ subsets: ["latin"] })`). */
export type GoogleFontFactory = (options?: GoogleFontOptions) => PlannedFont;

/** Creates a factory for any Google Fonts family name (`createGoogleFont("Noto Sans KR")`). */
export function createGoogleFont(family: string): GoogleFontFactory {
  return (options = {}) => googleFont(family, options);
}

/** Family names of the explicitly exported factories. */
export const GOOGLE_FONT_FAMILIES: ReadonlyArray<string> = [
  "Inter",
  "Roboto",
  "Roboto Mono",
  "Roboto Flex",
  "Open Sans",
  "Lato",
  "Montserrat",
  "Poppins",
  "Geist",
  "Geist Mono",
  "JetBrains Mono",
  "Source Sans 3",
  "Source Code Pro",
  "Noto Sans",
  "Noto Serif",
  "Noto Sans JP",
  "Nunito",
  "Nunito Sans",
  "Raleway",
  "Playfair Display",
  "Merriweather",
  "IBM Plex Sans",
  "IBM Plex Mono",
  "Fira Code",
  "Fira Sans",
  "Space Grotesk",
  "Space Mono",
  "DM Sans",
  "DM Serif Display",
  "Manrope",
  "Outfit",
  "Work Sans",
  "Rubik",
  "Ubuntu",
  "Ubuntu Mono",
  "Oswald",
  "Lora",
  "PT Sans",
  "PT Serif",
  "Mulish",
  "Karla",
  "Figtree",
  "Plus Jakarta Sans",
  "Sora",
  "Urbanist",
  "Onest",
  "Instrument Sans",
  "Instrument Serif",
  "Bricolage Grotesque",
  "Archivo",
  "Barlow",
  "Cabin",
  "Josefin Sans",
  "Quicksand",
  "Exo 2",
  "Red Hat Display",
  "Libre Franklin",
  "Libre Baskerville",
  "Crimson Text",
  "Crimson Pro",
  "EB Garamond",
  "Cormorant Garamond",
  "Inconsolata",
  "Overpass",
  "Hind",
  "Titillium Web",
  "Kanit",
  "Prompt",
  "Heebo",
  "Assistant",
  "Public Sans",
  "Lexend",
  "Be Vietnam Pro",
  "Albert Sans",
  "Schibsted Grotesk",
  "Atkinson Hyperlegible",
  "Chivo",
  "Epilogue",
  "Jost",
  "Syne",
  "Unbounded",
  "Fraunces",
  "Newsreader",
  "Literata",
  "Spectral",
  "Zilla Slab",
  "Roboto Slab",
  "Roboto Serif",
  "Cal Sans",
  "Anonymous Pro",
  "Courier Prime",
  "Victor Mono",
];

/** Google Fonts "Inter". */
export const Inter: GoogleFontFactory = createGoogleFont("Inter");
/** Google Fonts "Roboto". */
export const Roboto: GoogleFontFactory = createGoogleFont("Roboto");
/** Google Fonts "Roboto Mono". */
export const Roboto_Mono: GoogleFontFactory = createGoogleFont("Roboto Mono");
/** Google Fonts "Roboto Flex". */
export const Roboto_Flex: GoogleFontFactory = createGoogleFont("Roboto Flex");
/** Google Fonts "Open Sans". */
export const Open_Sans: GoogleFontFactory = createGoogleFont("Open Sans");
/** Google Fonts "Lato". */
export const Lato: GoogleFontFactory = createGoogleFont("Lato");
/** Google Fonts "Montserrat". */
export const Montserrat: GoogleFontFactory = createGoogleFont("Montserrat");
/** Google Fonts "Poppins". */
export const Poppins: GoogleFontFactory = createGoogleFont("Poppins");
/** Google Fonts "Geist". */
export const Geist: GoogleFontFactory = createGoogleFont("Geist");
/** Google Fonts "Geist Mono". */
export const Geist_Mono: GoogleFontFactory = createGoogleFont("Geist Mono");
/** Google Fonts "JetBrains Mono". */
export const JetBrains_Mono: GoogleFontFactory = createGoogleFont("JetBrains Mono");
/** Google Fonts "Source Sans 3". */
export const Source_Sans_3: GoogleFontFactory = createGoogleFont("Source Sans 3");
/** Google Fonts "Source Code Pro". */
export const Source_Code_Pro: GoogleFontFactory = createGoogleFont("Source Code Pro");
/** Google Fonts "Noto Sans". */
export const Noto_Sans: GoogleFontFactory = createGoogleFont("Noto Sans");
/** Google Fonts "Noto Serif". */
export const Noto_Serif: GoogleFontFactory = createGoogleFont("Noto Serif");
/** Google Fonts "Noto Sans JP". */
export const Noto_Sans_JP: GoogleFontFactory = createGoogleFont("Noto Sans JP");
/** Google Fonts "Nunito". */
export const Nunito: GoogleFontFactory = createGoogleFont("Nunito");
/** Google Fonts "Nunito Sans". */
export const Nunito_Sans: GoogleFontFactory = createGoogleFont("Nunito Sans");
/** Google Fonts "Raleway". */
export const Raleway: GoogleFontFactory = createGoogleFont("Raleway");
/** Google Fonts "Playfair Display". */
export const Playfair_Display: GoogleFontFactory = createGoogleFont("Playfair Display");
/** Google Fonts "Merriweather". */
export const Merriweather: GoogleFontFactory = createGoogleFont("Merriweather");
/** Google Fonts "IBM Plex Sans". */
export const IBM_Plex_Sans: GoogleFontFactory = createGoogleFont("IBM Plex Sans");
/** Google Fonts "IBM Plex Mono". */
export const IBM_Plex_Mono: GoogleFontFactory = createGoogleFont("IBM Plex Mono");
/** Google Fonts "Fira Code". */
export const Fira_Code: GoogleFontFactory = createGoogleFont("Fira Code");
/** Google Fonts "Fira Sans". */
export const Fira_Sans: GoogleFontFactory = createGoogleFont("Fira Sans");
/** Google Fonts "Space Grotesk". */
export const Space_Grotesk: GoogleFontFactory = createGoogleFont("Space Grotesk");
/** Google Fonts "Space Mono". */
export const Space_Mono: GoogleFontFactory = createGoogleFont("Space Mono");
/** Google Fonts "DM Sans". */
export const DM_Sans: GoogleFontFactory = createGoogleFont("DM Sans");
/** Google Fonts "DM Serif Display". */
export const DM_Serif_Display: GoogleFontFactory = createGoogleFont("DM Serif Display");
/** Google Fonts "Manrope". */
export const Manrope: GoogleFontFactory = createGoogleFont("Manrope");
/** Google Fonts "Outfit". */
export const Outfit: GoogleFontFactory = createGoogleFont("Outfit");
/** Google Fonts "Work Sans". */
export const Work_Sans: GoogleFontFactory = createGoogleFont("Work Sans");
/** Google Fonts "Rubik". */
export const Rubik: GoogleFontFactory = createGoogleFont("Rubik");
/** Google Fonts "Ubuntu". */
export const Ubuntu: GoogleFontFactory = createGoogleFont("Ubuntu");
/** Google Fonts "Ubuntu Mono". */
export const Ubuntu_Mono: GoogleFontFactory = createGoogleFont("Ubuntu Mono");
/** Google Fonts "Oswald". */
export const Oswald: GoogleFontFactory = createGoogleFont("Oswald");
/** Google Fonts "Lora". */
export const Lora: GoogleFontFactory = createGoogleFont("Lora");
/** Google Fonts "PT Sans". */
export const PT_Sans: GoogleFontFactory = createGoogleFont("PT Sans");
/** Google Fonts "PT Serif". */
export const PT_Serif: GoogleFontFactory = createGoogleFont("PT Serif");
/** Google Fonts "Mulish". */
export const Mulish: GoogleFontFactory = createGoogleFont("Mulish");
/** Google Fonts "Karla". */
export const Karla: GoogleFontFactory = createGoogleFont("Karla");
/** Google Fonts "Figtree". */
export const Figtree: GoogleFontFactory = createGoogleFont("Figtree");
/** Google Fonts "Plus Jakarta Sans". */
export const Plus_Jakarta_Sans: GoogleFontFactory = createGoogleFont("Plus Jakarta Sans");
/** Google Fonts "Sora". */
export const Sora: GoogleFontFactory = createGoogleFont("Sora");
/** Google Fonts "Urbanist". */
export const Urbanist: GoogleFontFactory = createGoogleFont("Urbanist");
/** Google Fonts "Onest". */
export const Onest: GoogleFontFactory = createGoogleFont("Onest");
/** Google Fonts "Instrument Sans". */
export const Instrument_Sans: GoogleFontFactory = createGoogleFont("Instrument Sans");
/** Google Fonts "Instrument Serif". */
export const Instrument_Serif: GoogleFontFactory = createGoogleFont("Instrument Serif");
/** Google Fonts "Bricolage Grotesque". */
export const Bricolage_Grotesque: GoogleFontFactory = createGoogleFont("Bricolage Grotesque");
/** Google Fonts "Archivo". */
export const Archivo: GoogleFontFactory = createGoogleFont("Archivo");
/** Google Fonts "Barlow". */
export const Barlow: GoogleFontFactory = createGoogleFont("Barlow");
/** Google Fonts "Cabin". */
export const Cabin: GoogleFontFactory = createGoogleFont("Cabin");
/** Google Fonts "Josefin Sans". */
export const Josefin_Sans: GoogleFontFactory = createGoogleFont("Josefin Sans");
/** Google Fonts "Quicksand". */
export const Quicksand: GoogleFontFactory = createGoogleFont("Quicksand");
/** Google Fonts "Exo 2". */
export const Exo_2: GoogleFontFactory = createGoogleFont("Exo 2");
/** Google Fonts "Red Hat Display". */
export const Red_Hat_Display: GoogleFontFactory = createGoogleFont("Red Hat Display");
/** Google Fonts "Libre Franklin". */
export const Libre_Franklin: GoogleFontFactory = createGoogleFont("Libre Franklin");
/** Google Fonts "Libre Baskerville". */
export const Libre_Baskerville: GoogleFontFactory = createGoogleFont("Libre Baskerville");
/** Google Fonts "Crimson Text". */
export const Crimson_Text: GoogleFontFactory = createGoogleFont("Crimson Text");
/** Google Fonts "Crimson Pro". */
export const Crimson_Pro: GoogleFontFactory = createGoogleFont("Crimson Pro");
/** Google Fonts "EB Garamond". */
export const EB_Garamond: GoogleFontFactory = createGoogleFont("EB Garamond");
/** Google Fonts "Cormorant Garamond". */
export const Cormorant_Garamond: GoogleFontFactory = createGoogleFont("Cormorant Garamond");
/** Google Fonts "Inconsolata". */
export const Inconsolata: GoogleFontFactory = createGoogleFont("Inconsolata");
/** Google Fonts "Overpass". */
export const Overpass: GoogleFontFactory = createGoogleFont("Overpass");
/** Google Fonts "Hind". */
export const Hind: GoogleFontFactory = createGoogleFont("Hind");
/** Google Fonts "Titillium Web". */
export const Titillium_Web: GoogleFontFactory = createGoogleFont("Titillium Web");
/** Google Fonts "Kanit". */
export const Kanit: GoogleFontFactory = createGoogleFont("Kanit");
/** Google Fonts "Prompt". */
export const Prompt: GoogleFontFactory = createGoogleFont("Prompt");
/** Google Fonts "Heebo". */
export const Heebo: GoogleFontFactory = createGoogleFont("Heebo");
/** Google Fonts "Assistant". */
export const Assistant: GoogleFontFactory = createGoogleFont("Assistant");
/** Google Fonts "Public Sans". */
export const Public_Sans: GoogleFontFactory = createGoogleFont("Public Sans");
/** Google Fonts "Lexend". */
export const Lexend: GoogleFontFactory = createGoogleFont("Lexend");
/** Google Fonts "Be Vietnam Pro". */
export const Be_Vietnam_Pro: GoogleFontFactory = createGoogleFont("Be Vietnam Pro");
/** Google Fonts "Albert Sans". */
export const Albert_Sans: GoogleFontFactory = createGoogleFont("Albert Sans");
/** Google Fonts "Schibsted Grotesk". */
export const Schibsted_Grotesk: GoogleFontFactory = createGoogleFont("Schibsted Grotesk");
/** Google Fonts "Atkinson Hyperlegible". */
export const Atkinson_Hyperlegible: GoogleFontFactory = createGoogleFont("Atkinson Hyperlegible");
/** Google Fonts "Chivo". */
export const Chivo: GoogleFontFactory = createGoogleFont("Chivo");
/** Google Fonts "Epilogue". */
export const Epilogue: GoogleFontFactory = createGoogleFont("Epilogue");
/** Google Fonts "Jost". */
export const Jost: GoogleFontFactory = createGoogleFont("Jost");
/** Google Fonts "Syne". */
export const Syne: GoogleFontFactory = createGoogleFont("Syne");
/** Google Fonts "Unbounded". */
export const Unbounded: GoogleFontFactory = createGoogleFont("Unbounded");
/** Google Fonts "Fraunces". */
export const Fraunces: GoogleFontFactory = createGoogleFont("Fraunces");
/** Google Fonts "Newsreader". */
export const Newsreader: GoogleFontFactory = createGoogleFont("Newsreader");
/** Google Fonts "Literata". */
export const Literata: GoogleFontFactory = createGoogleFont("Literata");
/** Google Fonts "Spectral". */
export const Spectral: GoogleFontFactory = createGoogleFont("Spectral");
/** Google Fonts "Zilla Slab". */
export const Zilla_Slab: GoogleFontFactory = createGoogleFont("Zilla Slab");
/** Google Fonts "Roboto Slab". */
export const Roboto_Slab: GoogleFontFactory = createGoogleFont("Roboto Slab");
/** Google Fonts "Roboto Serif". */
export const Roboto_Serif: GoogleFontFactory = createGoogleFont("Roboto Serif");
/** Google Fonts "Cal Sans". */
export const Cal_Sans: GoogleFontFactory = createGoogleFont("Cal Sans");
/** Google Fonts "Anonymous Pro". */
export const Anonymous_Pro: GoogleFontFactory = createGoogleFont("Anonymous Pro");
/** Google Fonts "Courier Prime". */
export const Courier_Prime: GoogleFontFactory = createGoogleFont("Courier Prime");
/** Google Fonts "Victor Mono". */
export const Victor_Mono: GoogleFontFactory = createGoogleFont("Victor Mono");
