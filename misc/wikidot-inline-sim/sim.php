<?php
// Minimal Text_Wiki inline pipeline (gabrys/wikidot rule regexes, rule order) + libtidy.
$tokens = [];
function tok($html) { global $tokens; $tokens[] = $html; return "\xFF" . (count($tokens)-1) . "\xFF"; }
function pair($src, $re, $open, $close, $grp=1) {
  return preg_replace_callback($re, function($m) use ($open,$close,$grp) {
    $o = is_callable($open) ? $open($m) : $open;
    return tok($o) . $m[$grp] . tok($close);
  }, $src);
}
function transform($src) {
  global $tokens; $tokens = [];
  $s = $src;
  $s = pair($s, '/\#\#(.+?)\|(.+?)\#\#/', function($m){ return '<span style="color: '.$m[1].'">'; }, '</span>', 2);
  $s = pair($s, "/\*\*([^\s\n](?:.*[^\s\n])?)\*\*/U", '<strong>', '</strong>');
  $s = pair($s, "/\/\/([^\s](?:.*[^\s])?)\/\//U", '<em>', '</em>');
  $s = pair($s, "/__([^\s](?:.*[^\s])?)__/U", '<span style="text-decoration: underline;">', '</span>');
  $s = pair($s, "/\-\-([^\s](?:.*[^\s])?)\-\-/U", '<span style="text-decoration: line-through;">', '</span>');
  $s = pair($s, "/{{({*?.*}*?)}}/U", '<tt>', '</tt>');
  $s = pair($s, "/\^\^([^\s](?:.*[^\s])?)\^\^/U", '<sup>', '</sup>');
  $s = pair($s, "/,,([^\s](?:.*[^\s])?),,/U", '<sub>', '</sub>');
  $s = preg_replace_callback('/\-\-/', function($m){ return tok('&#8212;'); }, $s);
  $s = htmlspecialchars($s, ENT_NOQUOTES, 'ISO-8859-1');
  $html = preg_replace_callback("/\xFF(\d+)\xFF/", function($m){ global $tokens; return $tokens[(int)$m[1]]; }, $s);
  return "<p>" . $html . "</p>";
}
function purify($code) {
  $head = '<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Strict//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd"><html xmlns="http://www.w3.org/1999/xhtml" xml:lang="en" lang="en">  <head>    <title>Just A Wrapper</title><meta http-equiv="content-type" content="text/html;charset=UTF-8"/>  </head> <!--wrapdelimiter--><body>';
  $tail = ' </body><!--wrapdelimiter--></html>';
  $c2 = tidy_parse_string($head.$code.$tail, ['indent'=>false,'output-xhtml'=>true,'wrap'=>0], 'UTF8');
  $arr = explode("<!--wrapdelimiter-->", (string)$c2);
  return str_replace(["<body>","</body>","\n"], "", $arr[1]);
}
$in = json_decode(file_get_contents('php://stdin'), true);
$out = [];
foreach ($in as $src) { $raw = transform($src); $out[] = ['raw'=>$raw, 'html'=>trim(purify($raw))]; }
echo json_encode($out, JSON_UNESCAPED_UNICODE|JSON_UNESCAPED_SLASHES);
