// Small stroke icons in the spirit of Warp's UI.
type P = { size?: number; className?: string };
const base = (size = 16) => ({
  width: size, height: size, viewBox: "0 0 24 24", fill: "none", stroke: "currentColor",
  strokeWidth: 1.8, strokeLinecap: "round" as const, strokeLinejoin: "round" as const,
});

export const IconSearch = ({ size, className }: P) => (<svg {...base(size)} className={className}><circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" /></svg>);
export const IconPlus = ({ size, className }: P) => (<svg {...base(size)} className={className}><path d="M12 5v14M5 12h14" /></svg>);
export const IconPlay = ({ size, className }: P) => (<svg {...base(size)} className={className}><path d="M7 5v14l11-7z" fill="currentColor" /></svg>);
export const IconStop = ({ size, className }: P) => (<svg {...base(size)} className={className}><rect x="6" y="6" width="12" height="12" rx="2" fill="currentColor" /></svg>);
export const IconGear = ({ size, className }: P) => (<svg {...base(size)} className={className}><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" /></svg>);
export const IconLogs = ({ size, className }: P) => (<svg {...base(size)} className={className}><path d="M4 6h16M4 12h10M4 18h13" /></svg>);
export const IconTerminal = ({ size, className }: P) => (<svg {...base(size)} className={className}><path d="m5 8 4 4-4 4M12 17h7" /></svg>);
export const IconSidebar = ({ size, className }: P) => (<svg {...base(size)} className={className}><rect x="3" y="4" width="18" height="16" rx="3" /><path d="M9 4v16" /></svg>);
export const IconExternal = ({ size, className }: P) => (<svg {...base(size)} className={className}><path d="M14 4h6v6M20 4l-9 9M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5" /></svg>);
export const IconCheck = ({ size, className }: P) => (<svg {...base(size)} className={className}><path d="m5 12 5 5 9-10" /></svg>);
export const IconX = ({ size, className }: P) => (<svg {...base(size)} className={className}><path d="M6 6l12 12M18 6 6 18" /></svg>);
export const IconFolder = ({ size, className }: P) => (<svg {...base(size)} className={className}><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /></svg>);
export const IconShield = ({ size, className }: P) => (<svg {...base(size)} className={className}><path d="M12 3 5 6v6c0 4 3 7.5 7 9 4-1.5 7-5 7-9V6z" /></svg>);
export const IconChevron = ({ size, className }: P) => (<svg {...base(size)} className={className}><path d="m9 6 6 6-6 6" /></svg>);
export const IconRefresh = ({ size, className }: P) => (<svg {...base(size)} className={className}><path d="M20 11a8 8 0 1 0-2.3 5.7M20 4v7h-7" /></svg>);
export const IconTrash = ({ size, className }: P) => (<svg {...base(size)} className={className}><path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3" /></svg>);
export const IconWarn = ({ size, className }: P) => (<svg {...base(size)} className={className}><path d="M12 4 2.5 20h19z" /><path d="M12 10v4M12 17.5v.5" /></svg>);
export const IconCompass = ({ size, className }: P) => (<svg {...base(size)} className={className}><circle cx="12" cy="12" r="9" /><path d="m15.5 8.5-2 5-5 2 2-5z" /></svg>);
