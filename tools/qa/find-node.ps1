param([string]$File, [string]$Name)
$j = Get-Content $File -Raw | ConvertFrom-Json
function Walk($n) {
    if ($n.name -eq $Name) { "{0} | {1}" -f $n.ref_id, $n.role }
    foreach ($c in $n.children) { Walk $c }
}
Walk $j.data.tree
