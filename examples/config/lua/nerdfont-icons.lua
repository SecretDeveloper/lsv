-- Nerd Font icon mappings for file extensions and folder names (single-cell glyphs)
-- Prefer these over emoji to avoid alignment issues in monospace terminals
return {
  extensions = {
    ["md,markdown,mdx"] = "", -- markdown
    rs   = "",  lua  = "󰢱",  toml = "",
    json = "",  js   = "",  ts   = "",
    html = "",  css  = "",
    yml  = "",  yaml = "",  ini = "",  conf = "",
    xml  = "",  csv  = "",  rst  = "",  txt = "",  log = "",
    -- grouped categories
    ["png,jpg,jpeg,webp,ico,heic,avif,gif,svg"] = "",   -- image
    { names = {"mp3","wav","flac","ogg","m4a"},        icon = "" }, -- audio
    { names = {"mp4","mkv","mov","webm","avi"},        icon = "" }, -- video
    ["zip,tar,gz,tgz,bz2,xz,rar,7z"] = "",                -- archive
    -- office/docs
    pdf  = "",
    doc  = "",  docx = "",
    xls  = "",  xlsx = "",
    ppt  = "",  pptx = "",
    -- more languages
    { names = {"c","h","cpp","hpp","cc"}, icon = "" },
    cs = "", java = "",
    { names = {"kt","kts"}, icon = "" },
    scala = "", swift = "", dart = "",
    rb = "", php = "", pl = "",
    sql = "",
    { names = {"jsx","tsx"}, icon = "" },
    py = "", go = "",
  },
  folders = {
    src   = "",
    docs  = "",
    ["test,tests"] = "",
    ["build,dist,out"] = "",
    node_modules = "",
    [".git"] = "",
    [".github"] = "",
    [".vscode"] = "",
    [".idea"] = "",
    target = "",
    bin = "",
    include = "",
    lib = "",
    assets = "", images = "",
    scripts = "",
    config = "",
    public = "", private = "",
    vendor = "", packages = "",
    examples = "", samples = "",
    data = "", db = "",
    migrations = "",
  },
}

