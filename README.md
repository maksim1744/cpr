# cpr

_Disclaimer: I did this for myself, so there are some constants in the code which you probably would like to change before building (mainly on top of [main.rs](src/main.rs)). One day I will move most of them to some settings file. Maybe._

## Installation
First install rust from [here](https://www.rust-lang.org/learn/get-started).
```
git clone https://github.com/maksim1744/cpr
cargo build --release
```
It will take some time and a couple of hundreds of Mb. After installation you can remove everything in `target/release` except for the `cpr.exe` (or whatever it is on linux).

If you are on linux, you may need to install GTK for drawing, you can read more [here](https://github.com/linebender/druid#linux).

## Usage
You can run `cpr` to see possible commands and `--help` is available with any command to learn more.

Here are the most useful ones:

#### `cpr init [params]`

Alias for `cpr mk [params] && cpr parse`. I have an alias for `cpt [task_name] [params] = mkdir [task_name] && cd [task_name] && cpr init [params]`, since `cd` is not really possible without scripts

#### `cpr mk`

Creates a file and writes a template to it. There are some options which you can see by running `cpr mk --help`. Templates should be in `.sublime-snippet` format, like this:
<details>
  <summary>Example</summary>
  
  ```xml
  <snippet>
      <content><![CDATA[/*
      author:  Maksim1744
      created: ${1:date}
  */

  int main() {
      ${0:}
  }
  ]]></content>
  everything below is not needed for cpr to work
      <tabTrigger>start</tabTrigger>
      <scope>source.c++</scope>
  </snippet>

  ```

  `${0:}` is for cursor position if you use sublime (or your editor supports opening a file like that `subl main.cpp:15:4`, where 15 is a line number and 4 is a column number).

  `${1:date}` will be replaced with current date and time.

  You can try to change something in the end of `make_file` function in [main.rs](src/main.rs), including the last line, which opens your editor
  
</details>

Templates should be in a folder `TEMPLATE_PATH/[language]/*.sublime-snippet`. For example, default demplate for `c++` should be located like that: `TEMPLATE_PATH/C++/start.sublime-snippet`. You can change `TEMPLATE_PATH` in [main.rs](src/main.rs).

#### `cpr parse`

Parses samples. Works beautifully for codeforces, acceptable for atcoder and cses, sometimes for codechef. To work you either have to specify url for the task, or have a resource name with contest and task number in the path of current folder. For example, these paths to current directory would work:
```
.../Codeforces/.../1234/A
.../Atcoder/.../agc123/a
.../Cses/.../problemset/.../1234

With codefoces also works like this:
.../Codeforces/.../1234A
```

For parsing atcoder during live contest you have to enter you login and password in the `settings.json` file (read below)

Note that there is a much better tool [Competitive Companion](https://github.com/jmerle/competitive-companion).

#### `cpr test`
Suppose your folder for problem A looks like this:
```
A
|- main.cpp
|- main.exe
|- in1
|- in8
|- ans1
```

That means that you have inputs for tests 1 and 8 and answer for test 1. If you just run `cpr test`, it will run `main.exe` on tests 1 and 8, save outputs to `out1` and `out8` and compare `out1` with `ans1` to check for WA. All RE and TLE will be caught. As usual, see all options with `cpr test --help`. For example, you can specify `eps` to check floating-point problems, or use `check.exe` to check answer insted of blindly comparing them.

#### `cpr mktest`
Create test without parsing or manually creating files. You have to first write input, then answer and separate them with a single line with a character \`
<details>
  <summary>Example</summary>

A test for A+B problem

```
.../1234/A> cpr mktest
3 4
`
7
`
.../1234/A>
```

</details>

#### `cpr stress`
Performs stress-testing. You need `easy.exe` as correct solution and `gen.exe` as a generator. `cpr` will run `gen.exe [i]` on iteration `i`, so you can use it as fixed seed. When test found, all needed info will be stored in files `in`, `out`, `ans`. And then you can easily make a test out of this with `cpr mktest -0`. As with `cpr test`, you can write checker instead of bruteforce.

#### `cpr interact`
Test interactive problems. You should write code for judge in `interact.exe`. On test `i` it will be run as `interact.exe [i]` just like generator. It should return 0 if everything is correct and something else otherwise. Both your solution and `interact` can write to stderr, it will be shown to you.

#### `cpr draw`
It can draw tree, graphs, a bunch of points or a matrix. stdin should look like this:
<details>
  <summary>Tree example</summary>

```
5
1 2
1 3
3 4
3 5
```
and then `cpr draw tree <in`

or

```
5
1 2 q
1 3 w
3 4 akdfj ajf lask
3 5 j
```
and then `cpr draw tree -ei <in`

or

```
5
v1 v2 v33333 __v4 blablabla
1 2
1 3
3 4
3 5
```
and then `cpr draw tree -vi <in`

</details>
<details>
  <summary>Graph example</summary>

```
4 5
1 2
1 3
2 3
4 3
4 2
```
and then `cpr draw graph <in`

Or with `-vi` or `-ei` similarly to `cpr draw tree`

</details>
<details>
  <summary>Points example</summary>

```
3
0 0
0.5 1.8
2.3 2.8
```
and then `cpr draw pts <in`

</details>
<details>
  <summary>Matrix example</summary>

```
2 3
A B C
D EEEEEEEE 123
```
and then `cpr draw matrix <in`

or

```
3 3
XO.
OX.
..X
```
and then `cpr draw matrix -c <in`

</details>

## `check.exe`

Whenever I mention a file `check.exe`, it will be run on a concatenation of input and your answer. It should return 0 if everything is correct and any other value otherwise. Also it can write something to stdout. For example, for A+B problem, your `check.cpp` may look like this:
<details>
  <summary>Example</summary>

```cpp
int main() {
    int a, b;
    cin >> a >> b;
    int ans;
    cin >> ans;
    if (ans != a + b) {
        cout << "wrong sum, " << a << " + " << b << " != " << ans << endl;
        return 1;
    }
    return 0;
}
```

</details>

## `settings.json`

For now this file is only for passwords for codeforces and atcoder. It looks like this:
<details>
  <summary>Example</summary>

```json
{
    "auth": {
        "codeforces": {
            "login": "your_name",
            "password": "your_password"
        },
        "atcoder": {
            "login": "your_name",
            "password": "your_password"
        }
    }
}
```

</details>

I don't promise that this is completely safe or that it will not leak anything accidentally, so use it on your own risk.

## `FAQ`

#### I'm a python user and I get `Case #1:    Error when starting process ["main.py"]` when I try to run `cpr test main.py`
That's because `cpr test` (and any other similar function) accepts only executables. With python you have to write `cpr test "python main.py"`

#### I'm on linux/mac and something doesn't work.
Yeah, about that, I don't know how to help you, I tested only on Windows 10. If it looks like a general problem (not dependent on OS), you can contact me (create an issue, for example)
