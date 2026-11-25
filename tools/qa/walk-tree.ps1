function Find-Node($n,[string]$name){ if($n.name -eq $name){return $n}; foreach($c in $n.children){ $r=Find-Node $c $name; if($r){return $r} }; return $null }
$j=Get-Content D:\Workspace\_s.json -Raw | ConvertFrom-Json
$n=Find-Node $j.data.tree '显示 / 隐藏桌面歌词'
if($n){ "ref=$($n.ref_id) states=$($n.states)" } else { "not found" }
