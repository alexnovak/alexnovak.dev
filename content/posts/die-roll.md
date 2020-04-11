---
date: 2020-03-22
title: Die Roll
author: Alex
draft: false
---

A few weeks ago, a friend and I were taking a walk when he shared an old interview question.

> Generate a random die-roll. One through six.

This was a lead in to a longer, harder question, but was there to get the ball rolling.
Both he and the interviewer agreed it was fine to just grab some random integer, so some randum unsigned 32 bit integer,
apply a modulo 6, and then add one. Okay, sounds good. You get a random number between zero and five, 
you add one, and you get one to six.

Let's look at this quickly and transparently in Rust just so we have an example of what we're talking about. 

```rust
// In main.rs
use std::fs::File;
use std::io::{Error, Read};

// Slapdash code to grab a random u32 value.
fn get_random_int() -> Result<u32, Error> {
    let mut urandom = File::open("/dev/urandom")?;
    let mut buf = [0_u8; 4];
    urandom.read(&mut buf)?;
    // Rust hackery to turn four u8s into a u32.
    // Here we assume the bytes are little endian, but it really doesn't matter.
    let res = u32::from_le_bytes(buf);
    Ok(res)
}

fn main() -> Result<(), Error> {
    let random = get_random_int()?;
    println!("Random number is: {}", random % 6 + 1);
    Ok(())
}
```

The above code reaches out to some source of randomness (in this case `/dev/urandom`), and gets four bytes.
It then casts these four bytes as an unsigned integer, and performs our little "die roll from a random integer" algorithm.
If we run this a few times, we'll see we get expected results.

```shell
$ cargo run
    Finished dev [unoptimized + debuginfo] target(s) in 0.29s
      Running `target/debug/random`
Random number is: 6
$ cargo run
    Finished dev [unoptimized + debuginfo] target(s) in 0.02s
      Running `target/debug/random`
Random number is: 4
$ cargo run
    Finished dev [unoptimized + debuginfo] target(s) in 0.02s
      Running `target/debug/random`
Random number is: 5
```

Cool. Works. Time to ship it to production.

Eh -- maybe not.

There's something really subtly wrong about this. To point it out I'll add some boilerplate to help
illustrate the point. Let's start with two dumb functions.

```rust
// in main.rs
use std::collections::BTreeMap;

...

// This is just our die roll as a function.
fn die_roll(random_int: u32) -> u32 {
    random_int % 6 + 1
}

// Run some specified number of trials of our die roll experiment,
// keeping the frequency of our results in a map.
fn get_longrunning_frequency(trials: u32) -> Result<BTreeMap<u32, u32>, Error> {
    let mut frequency: BTreeMap<u32, u32> = BTreeMap::new();
    for _ in 0..trials {
        let number = get_random_int()?;
        let roll = die_roll(number);
        // Little weird rust hack, if the entry for a value doesn't exist, insert 0.
        let counter = frequency.entry(roll).or_insert(0);
        *counter += 1;
    }
    Ok(frequency)
}
```

These give us an easy way of getting our die roll frequencies. (We use a BTreeMap because it sorts the keys
-- which is just for the sake of our eyes when printing.) Let's also modify main to take advantage of our changes.

```rust
fn main() -> Result<(), Error> {
    let trials = 10_000;
    let frequency = get_longrunning_frequency(trials)?;
    for (value, appearances) in frequency {
        println!(
            "Value: {}, frequency: {}, percentage: {}",
            value,
            appearances,
            appearances as f64 / trials as f64
        );
    }
    Ok(())
}
```

Running this does what you'd expect. We'll see percentages for how often each roll occurred,
and in this case they're all roughly equal.

```shell
$ cargo run
    Finished dev [unoptimized + debuginfo] target(s) in 0.02s
         Running `target/debug/random`
         Value: 1, frequency: 1652, percentage: 0.1652
         Value: 2, frequency: 1706, percentage: 0.1706
         Value: 3, frequency: 1655, percentage: 0.1655
         Value: 4, frequency: 1657, percentage: 0.1657
         Value: 5, frequency: 1637, percentage: 0.1637
         Value: 6, frequency: 1693, percentage: 0.1693
```

But this isn't the point I want to make. Right now we're using 32 random bits to generate this number.
That's a bit wasteful isn't it? As good engineers, we hate waste! So let's modify code to generate only 
three bits instead, and use that to generate our die roll. Just make a quick change to `get_random_int`.

```rust
fn get_random_int() -> Result<u32, Error> {
    let urandom = File::open("/dev/urandom")?;
    // Take gives us a handle that, when read, gives us only n bytes. 1 in this case.
    let mut handle = urandom.take(1);
    let mut buf = [0_u8; 4];
    handle.read(&mut buf)?;
    // Snip off all but the last three bits.
    // I won't lie that it took me a few guesses for which bits to snip.
    // Endianness is hard.
    buf[0] &= 0b00000111;
    let res = u32::from_le_bytes(buf);
    Ok(res)
}
```

Perfect! Now we can run this again, and we'll feel fine about throwing away all those useless bits.

```shell
$ cargo run
   Compiling random v0.1.0 (/tmp/random)
       Finished dev [unoptimized + debuginfo] target(s) in 0.41s
            Running `target/debug/random`
            Value: 1, frequency: 2431, percentage: 0.2431
            Value: 2, frequency: 2477, percentage: 0.2477
            Value: 3, frequency: 1274, percentage: 0.1274
            Value: 4, frequency: 1264, percentage: 0.1264
            Value: 5, frequency: 1256, percentage: 0.1256
            Value: 6, frequency: 1298, percentage: 0.1298
```

Huh... That's unexpected. Both 1 and 2 are appearing nearly twice as frequently as all others. 
Last time I checked, dice are supposed to give equal chance to each number, so this doesn't work.
But why is this happening? We're doing the same thing we did with 32 bits -- just being more economic.

Maybe we need to think about this a little more carefully. Let's work backwards and try using some of that
math stuff I've heard so much about. Our goal is to produce a number between one and six. So we want some function $f$
that's randomly pulling values from a domain $D$ and produces our die roll. We can *pretty much* assume $D$ is uniformly
distributed over zero through seven for three bits.

$$ f: \\{0, 1, 2, 3, 4, 5, 6, 7\\} \rightarrow \\{1, 2, 3, 4, 5, 6\\} $$

As it stands right now, we've defined $f(x) = \(x \mod 6\) + 1$,
where $x \in \\{0, ..., 7\\}$. The set of all unsigned integers expressible in three bits.
To make things a little clearer, let's just draw out the function inputs and outputs.

$$ \begin\{pmatrix\}
0 & \rightarrow & 1\\\\
1 & \rightarrow & 2\\\\
2 & \rightarrow & 3\\\\
3 & \rightarrow & 4\\\\
4 & \rightarrow & 5\\\\
5 & \rightarrow & 6\\\\
6 & \rightarrow & 1\\\\
7 & \rightarrow & 2\\\\
\end\{pmatrix\} $$ 

*Now* I get it. The problem is with our domain. If our source of randomness gives us something from zero through six
then all things go according to plan, but because modulo will wrap around, we end up with twice the chance to roll
a one or a two. Our actual probabilities are

$$ P\(1\) = \frac\{2\}\{8\} = .25\\\\
P\(2\) = \frac\{2\}\{8\} = .25\\\\
P\(3\) = \frac\{1\}\{8\} = .125\\\\
P\(4\) = \frac\{1\}\{8\} = .125\\\\
P\(5\) = \frac\{1\}\{8\} = .125\\\\
P\(6\) = \frac\{1\}\{8\} = .125\\\\ $$

If our random number source's size was divisible by six, this wouldn't be a problem. We would be able to 
map an equal number of inputs to each desired output. Here's where we run into a problem however, our random number
source *can't* be divisible by six. We're constrained to base two numbers. In the process of generating a random number,
it's always going to be one of $2^n$ many options. This is the problem with any method that relies on modulus. 
The one workaround for modulus, and why you'll see this method recommended so often, is to just use more random bits.
With only three random bits, this problem is *very* apparent, but when working with thirty two? Our probabilities shift a lot.

$$ P\(1\) = \frac\{715827883\}\{2^\{32\}\} \approx .1666666667\\\\
P\(2\) = \frac\{715827883\}\{2^\{32\}\} \approx .1666666667\\\\
P\(3\) = \frac\{715827883\}\{2^\{32\}\} \approx .1666666667\\\\
P\(4\) = \frac\{715827883\}\{2^\{32\}\} \approx .1666666667\\\\
P\(5\) = \frac\{715827882\}\{2^\{32\}\} \approx .1666666665\\\\
P\(6\) = \frac\{715827882\}\{2^\{32\}\} \approx .1666666665\\\\ $$

Like how our three bits overshot a multiple of six by two, thirty two bits overshoots a multiple of six by four, meaning
the first four possibilities are ever so lightly more likely.

In the vast majority of cases, this method works completely fine and you shouldn't sweat it. We live in the modern age,
32 random bits are easy to come by, and it'll take somewhere in the range of $10^\{10\}$ to $10^\{11\}$ runs to notice
that five and six are *slightly* less likely. But for this method, size is still a critical factor. It works for a six
sided die, but what if we had a really honkin' big die? Maybe one with $2^\{31\}-1$ many sides? Or we're working in 
a security space, where even small deviations from uniformity can cause eyebrows to raise. What then?

The root of the problem is the mismatch in the size of the domain, and the size of the range. Unless they match, or
the domain's size is a multiple of the range, we're stuck. We're going to have a few extra values that we have to map 
to *something*. It's not like we can throw away numbers, right?

Let's throw away some numbers.

Consider the following alteration to our old buddy $f$. Originally, it just performed modulus and added one, 
but what if we considered the following instead?

$$ f(x) = \begin\{cases\} x + 1 & x \in \\{0, 1, 2, 3, 4, 5 \\} \\ \newline f(x) & x \in \\{6, 7\\} \end\{cases\} $$

This is a formalization of "If I didn't get something I can map to one through six, try again." So, throw away my current
random bits, get another set of random bits, and hope that I'll be able to work with them. We can prove that this method works.
I've been pretty abusive with my mathematical notation so far, but when I write $P(n)$, let's take that to mean
$P(n) = P(f(x) = n \mid x \sim U\\{0, 7\\})$. Or in plain English "The probability that the output of $f(x)$ is
$n$, given that its input $x$ is uniformly drawn from zero through seven." Math takes too many words, there's a reason
I dumped it for computers.

We can prove that this function gives us a nice even die roll by showing that $P(n) = \frac\{1}\{6}$ for any $n$ in 
$\\{0,...,7\\}$. Let's start by arbitrarily fixing $n$. On our first draw, we have a one in eight chance of getting it right,
and a two in eight chance that we need to try again. On that second trial, we again have a one in eight chance of 
getting it right, and a two in eight chance of trying again, this repeats ad infinitum.

$$P(n) = \frac\{1\}\{8\} + \frac\{2\}\{8\}P(n \mid \mbox{first attempt failed}) \Rightarrow \\\\
P(n) = \frac\{1\}\{8\} + \frac\{2\}\{8\}P(n) \Rightarrow \\\\
\frac\{6\}\{8\}P(n) = \frac\{1\}\{8\} \Rightarrow \\\\
P(n) = \frac\{1\}\{6\}
$$
Neat, huh? We can just keep trying and actually get the desired results. In our quick proof, we use the fact that
our future draws are all independent of previous draws. Independance is really important in probability, but basically
boils down to whether or not the past matters for future random events. Past coin flips don't impact future coinfips,
but a well performing stock portfolio has a higher likelihood of hitting a high number than a low performing stock 
portfolio.

Back to the matter at hand, let's try out our new method to see whether my degree was worth it.
We can make small edits to our `die_roll` and `get_longrunning_frequency` functions to get the job done.

```rust
// In main.rs

// Modify to return result instead.
fn die_roll() -> Result<u32, Error> {
    // Get the random number inside this function instead.
    let mut random_int = get_random_int()?;
    // Here's the magic! Just keep trying.
    while random_int > 5 {
        random_int = get_random_int()?;
    }
    Ok(random_int % 6 + 1)
}

fn get_longrunning_frequency(trials: u32) -> Result<BTreeMap<u32, u32>, Error> {
    let mut frequency: BTreeMap<u32, u32> = BTreeMap::new();
    for _ in 0..trials {
        
        let roll = die_roll()?;
        let counter = frequency.entry(roll).or_insert(0);
        *counter += 1;
    }
    Ok(frequency)
}
```

Running it, we see

```shell
$ cargo run
    Finished dev [unoptimized + debuginfo] target(s) in 0.02s
         Running `target/debug/random`
         Value: 1, frequency: 1671, percentage: 0.1671
         Value: 2, frequency: 1705, percentage: 0.1705
         Value: 3, frequency: 1665, percentage: 0.1665
         Value: 4, frequency: 1668, percentage: 0.1668
         Value: 5, frequency: 1648, percentage: 0.1648
         Value: 6, frequency: 1643, percentage: 0.1643
```

Much better! These results are much much closer to one sixth (about 1.6666). We only did a quick proof for one
through six, but you can pretty easily extend this method for any upper boundary. For example, if we needed a one hundred
sided die, we just modify our code to collect seven bits instead of three 
($2^7 = 128$ is the smallest power of two greater than 100), and to keep trying if the number is greater than one hundred.

"*But Alex*," you cry, "won't this nondeterministic while loop in my code cause weird performance issues?" Good question!
And there's actually a pretty satisfying answer. Let's rephrase the question as "How many times will I have to request new
bits before this succeeds?" Questions like "how may discrete trials before I see one success?" are answered by a geometric
distribution.

We'll say that the number of attempts before we get a valid random number (one within our one to $n$ range) is $T$.
The probability that $T$ is equal to $k$, where $k\in \\{1,2,3,...\\}$ is

$$P(T=k) = (1-p)^\{k-1\}p$$
Where $p$ is the likelihood we get a valid random number in any given trial. But what is the likelihood we get a valid 
random number? In the case our six sided die, we had a three in four chance $\left(\frac\{6\}\{8\}\right)$, but this is a
pretty generous case. The worst case is $2^n + 1$ for some $n$. In order to generate a number between one and $2^n + 1$, we
will need at least $2^\{n+1\}$ random bits, meaning that our odds of getting a valid random number are 
$\frac\{2^n+1\}\{2^\{n+1\}\}$, greater than one half. At very large values of $n$, this will approach, but never dip below
one half.

This is already looking pretty good for us. In the worst case, we still have a coin flip's chance of getting a valid
random number. Let's assume the worst, that our odds of getting a valid random number is around fifty percent. We can now
start to see roughly how long it's going to take. For example, what are the odds we're going to see it in the first three
attempts?

$$P(T \leq 3) = P(T = 1) + P(T = 2) + P(T = 3) = \\\\
\frac\{1\}\{2\} + \left(\frac\{1\}\{2\}\right)^2 + \left(\frac\{1\}\{2\}\right)^3 = .875$$

In the worst possible case, you have an 87.5% chance you'll see it in the first three attempts. An extra attempt raises this
even further to 93.75%. 

There's more to unpack here, but this post is already going much longer than I ever anticipated. To finish thing off,
we started with a simple problem, find a random number between one and $n$, discovered an interesting limitation
because of our representation, and took a peek at the properties of a workaround.

I hope this was an insightful read! Any questions, concerns or complaints can be directed to alex[[at]]alexnovak.dev


