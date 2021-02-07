require('io')
require('os')
 
function isdir(path)
	local fh = io.open(path, 'r')
	_, _, errno = fh:read()
	return errno == 21
end
 
function ls(dir)
	local d = dir
	if (type(dir) == type(nil)) then d = '' end
	local fh = io.popen('ls -a '..d)
	local retvl = {}
	local idx = 0
	for ln in fh:lines() do 
		retvl[idx] = ln
		idx = idx + 1
	end
	return retvl
end

execrep = '/usr/bin/flatpak run --branch=stable --arch=x86_64 --command=/app/bin/steam-wrapper com.valvesoftware.Steam'
function readfile(path)
	local ret = {}
	local curtitle = nil
	for ln in io.lines(path) do
		_start, _end_, title = string.find(ln, '%[([^%]]*)%]' )
		if title then
			ret[title] = {}
			curtitle = title
		else
			_start, _end, k, v = string.find(ln, "([^=]*)=(.*)")
			if not ret[curtitle][k] then 
				ret[curtitle][k] = {}
			end
			if k == 'Exec' then 
				v = string.gsub(v, 'steam', execrep, 1)
			end
			ret[curtitle][k] = v
		end
	end
	return ret
end

function getents()
	base = os.getenv('HOME')..'/.var/app/com.valvesoftware.Steam/Desktop'
	local ret = {}
	local idx = 1
	for _, ent in ipairs(ls(base)) do 
		if not isdir(base..'/'..ent) then 
			local tm = readfile(base ..'/'..ent)
			local name = tm["Desktop Entry"].Name
			ret[idx] = entry {
				name = name, 
				exec = tm["Desktop Entry"].Exec,
				search_terms = {
					name, 'Steam', 'Game', 
				}
			}
			idx = idx + 1
		end
	end
	return ret
end


plugin {
	name = "Steam",
	entries = getents(),
}