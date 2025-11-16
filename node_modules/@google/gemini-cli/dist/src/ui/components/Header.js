import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { Box, Text } from 'ink';
import Gradient from 'ink-gradient';
import { theme } from '../semantic-colors.js';
import { shortAsciiLogo, longAsciiLogo, tinyAsciiLogo, shortAsciiLogoIde, longAsciiLogoIde, tinyAsciiLogoIde, } from './AsciiArt.js';
import { getAsciiArtWidth } from '../utils/textUtils.js';
import { useTerminalSize } from '../hooks/useTerminalSize.js';
import { getTerminalProgram } from '../utils/terminalSetup.js';
const ThemedGradient = ({ children, }) => {
    const gradient = theme.ui.gradient;
    if (gradient && gradient.length >= 2) {
        return (_jsx(Gradient, { colors: gradient, children: _jsx(Text, { children: children }) }));
    }
    if (gradient && gradient.length === 1) {
        return _jsx(Text, { color: gradient[0], children: children });
    }
    return _jsx(Text, { children: children });
};
export const Header = ({ customAsciiArt, version, nightly, }) => {
    const { columns: terminalWidth } = useTerminalSize();
    const isIde = getTerminalProgram();
    let displayTitle;
    const widthOfLongLogo = getAsciiArtWidth(longAsciiLogo);
    const widthOfShortLogo = getAsciiArtWidth(shortAsciiLogo);
    if (customAsciiArt) {
        displayTitle = customAsciiArt;
    }
    else if (terminalWidth >= widthOfLongLogo) {
        displayTitle = isIde ? longAsciiLogoIde : longAsciiLogo;
    }
    else if (terminalWidth >= widthOfShortLogo) {
        displayTitle = isIde ? shortAsciiLogoIde : shortAsciiLogo;
    }
    else {
        displayTitle = isIde ? tinyAsciiLogoIde : tinyAsciiLogo;
    }
    const artWidth = getAsciiArtWidth(displayTitle);
    return (_jsxs(Box, { alignItems: "flex-start", width: artWidth, flexShrink: 0, flexDirection: "column", children: [_jsx(ThemedGradient, { children: displayTitle }), nightly && (_jsx(Box, { width: "100%", flexDirection: "row", justifyContent: "flex-end", children: _jsxs(ThemedGradient, { children: ["v", version] }) }))] }));
};
//# sourceMappingURL=Header.js.map