require('io')
require('os')

function config_dir()
    local ret = os.getenv('XDG_CONFIG_DIR')
    if not ret then
        local home = os.getenv('HOME')
        if home then
            return home .. '/.config'
        else
            return nil
        end
    else
        return ret
    end
end

function filext(raw)
    prev = nil
    for w in string.gmatch(raw, "%.([^%.]*)$") do
        prev = w
    end
    if prev == string.sub(raw, 2, -1) then
        return nil
    else
        return prev
    end
end

function parse_file(path)
    local env = {}
    local chunk, err = loadfile(path, 't', env)
    if not err then
        chunk()
    end
    return env, err
end

function ls(dir)
    local d = dir
    if (type(dir) == type(nil)) then
        d = ''
    end
    local fh = io.popen('ls -a ' .. d)
    local retvl = {}
    local idx = 0
    for ln in fh:lines() do
        if not (ln == '..') and not (ln == '.') then
            retvl[idx] = ln
            idx = idx + 1
        end
    end
    return retvl
end

function parse_ext(raw)
    local retvl = {}
    for part in string.gmatch(raw, "([^|]*)") do
        if #part > 0 then
            table.insert(retvl, part)
        end
    end
    return retvl
end

function get_core_data()
    local config_path = config_dir() .. '/retroarch/retroarch.cfg'
    local cfg = parse_file(config_path)

    local cores = {}
    cfg.libretro_directory = string.gsub(cfg.libretro_directory, '~', os.getenv('HOME'))
    for idx, subdir in ipairs(ls(cfg.libretro_directory)) do
        local corepath = cfg.libretro_directory .. '/' .. subdir
        local corename = string.gmatch(subdir, "(.*)%.so$")()
        cores[corename] = {
            path = corepath
        }
    end

    cfg.libretro_info_path = string.gsub(cfg.libretro_info_path, '~', os.getenv('HOME'))
    for idx, subdir in ipairs(ls(cfg.libretro_info_path)) do
        local fpath = cfg.libretro_info_path .. '/' .. subdir
        local corename = string.gmatch(subdir, "(.*)%.info$")()
        if not (cores[corename] == nil) then
            loaded = parse_file(fpath)
            for k, v in pairs(loaded) do
                if k == 'supported_extensions' then
                    cores[corename][k] = parse_ext(v)
                else
                    cores[corename][k] = v
                end
            end
        end
    end
    return cores
end

function by_extension(cores)
    local retvl = {}
    for core, data in pairs(cores) do
        if data.supported_extensions == nil then
            data.supported_extensions = {}
        end
        for idx, extn in pairs(data.supported_extensions) do
            if retvl[extn] == nil then
                retvl[extn] = {}
            end
            table.insert(retvl[extn], {
                [core] = data
            })
        end
    end
    return retvl
end

function by_systemid(cores)
    local retvl = {}
    for core, data in pairs(cores) do
        if data.systemid == nil then
            data.systemid = ''
        end
        if retvl[data.systemid] == nil then
            retvl[data.systemid] = {}
        end
        table.insert(retvl[data.systemid], {
            [core] = data
        })
    end
    return retvl
end

function get_roms()
    local ROM_ROOT = os.getenv('HOME') .. '/roms'
    local retvl = {}
    for idx, dir in pairs(ls(ROM_ROOT)) do
        local parent = ROM_ROOT .. '/' .. dir
        for jdx, child in pairs(ls(parent)) do
            local rompath = parent .. '/' .. child
            local ext = filext(rompath)
            local name = child
            if ext then
                name = string.sub(child, 1, #child - #ext - 1)
            end
            table.insert(retvl, {
                name = name,
                path = rompath,
                ext = ext,
                systemid = dir
            })
        end
    end
    return retvl
end

function getdata()
    local cores = get_core_data()
    local bext = by_extension(cores)
    local bsys = by_systemid(cores)
    local retvl = {}
    for idx, romdata in pairs(get_roms()) do

        datacores = {}

        if romdata.ext == 'zip' or romdata.ext == '7z' then
            datacores = bsys[romdata.systemid]
        else
            datacores = bext[romdata.ext]
        end
        if datacores == nil then
            datacores = {}
        end
        if romdata.systemid == 'mame' then
            for idx, core in pairs(bsys['fb_alpha']) do
                table.insert(datacores, core)
            end
        end
        if romdata.systemid == 'fb_alpha' then
            for idx, core in pairs(bsys['mame']) do
                table.insert(datacores, core)
            end
        end

        if #datacores == 1 then
            core = datacores[1]
            for corename, data in pairs(core) do
                table.insert(retvl, entry {
                    name = romdata.name,
                    exec = "retroarch -L " .. data.path .. ' ' .. romdata.path,
                    search_terms = {romdata.name, romdata.ext, romdata.systemid, corename}
                })
            end
        else
            local head_entry = entry {
                name = romdata.name,
                exec = "",
                search_terms = {romdata.name, romdata.ext, romdata.systemid},
                children = {}
            }
            for idx, core in pairs(datacores) do
                for corename, data in pairs(core) do
                    local curexec = "retroarch -L \"" .. data.path .. '" "' .. romdata.path..'"'
                    table.insert(head_entry.children, entry {
                        name = corename,
                        exec = curexec,
                        search_terms = {romdata.name, romdata.ext, romdata.systemid, corename}
                    })
                    table.insert(head_entry.search_terms, corename)
                    if head_entry.exec == nil or #head_entry.exec == 0 then
                        head_entry.exec = curexec
                    end
                end
            end
            table.insert(retvl, head_entry)
        end
    end
    return retvl
end

plugin {
    name = "Retroarch ROMS", 
    entries = getdata()
}
