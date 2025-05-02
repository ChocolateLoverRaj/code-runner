This document is not really specific to a custom OS. It is programming-related in general.

There are apps backed by data. Examples:
- List App
- Text Editor
- Presentation Maker
- Video Editor

Some apps let you:
- Save and open/import files
- Live-update the state when the file changes
- Synchronize with its own cloud service

However, there is no perfect app, besides some text editors, I guess. In my opinion, the perfect synchronization includes:
- Easily self-hostable, with or without a server that is always on
- Doesn't need any proprietary software
- Doesn't use exessive CPU, Network, or Storage space
- Minimizes the amount of manual conflict fixing needed
- Works really good offline
- Works for real-time collaboration and synchronizes really fast
- Ability to synchronize by sharing files manually

## Basic Counter App
Imagine an app which just has a counter and a button, and every time you press the button the counter increases by 1. The counter is a `u64`.
```rs
struct Data(u64)
```

## Storing the counter
Ez. You just need to store the `u64` in a file or something.

## Sharing as "read-only"
You just send the file which contains the `u64` to your friend, and then your friend can open it and it will be read-only (the increase counter button will be disabled). You can always keep sending your friend the updated file which will have the new counter value.

## Sharing as read & write
You can send the file to your friend, and then your friend can edit the file. However, it is a problem if your friend send it back and you made edits.

Scenario:
- Initial counter: 0
- You send it to your friend
- Your friend: +1
- You: +1
- Your friend send you the file which is `1`
- Now both of your counters are `1` but they should both be `2` because it should count both of your button clicks.

## A Git commit-like structure
We could have a "commit" for every time you click the button.
```rs
struct Data {
  commits: HashSet<Commit>
}

struct Commit {
  id: u64,
}
```
When you share files, you can either share every commit.

## Merging commits
You can just import the commit ids that you don't already have. This also prevents issues of duplicate imports.

## Verifying commits
You can of course avoid having to verify commits if you trust the source that you are getting them from. In the case of sending commits through a USB drive, you can make sure that only your friend with write permission gives you the USB drive. However, for the convenience of synchronization, it is convenient if people without write permission are also able to send you the latest updates. Imagine this scenario:

- A has RW
- B has RW
- C has R
- B makes an edit
- B sends C the edit, and A is not online at that time
- Later, A and C are online, but B isn't online. C can send the latest edits to A, but as of now, A cannot accept them because it doesn't know if C just made them up. C is not allowed to make them up because it has  read-only access.

So we can make it so that everyone has a list of people who can write. Then every commit can be signed and stored signed. In the scenario, A can verify that the commits were made by B.

```rs
struct Data {
  editors: HashSet<Signature>,
  commits: HashSet<Signed<Commit>>
}

struct Commit {
  id: u64,
}
```

## Adding editors
Adding viewers is easy. You just give them a list of commits and the list of editors. But how do you add editors? You can make each editor in the list of editor signed by an existing editor. Scenario:
- A and B start as editors
- A adds C as an editor
- C sends its commits to B, and B trusts C because it sees that A signed that C is now an editor.

## Removing editors
If you remove editors, do you want to keep or delete edits made by the editor you are removing?

If you want to delete edits it's easy. Since the signature of each commit can identify who made the edit, you can just ignore edits made by that person.

If you want to keep edits made until now but not allow future edits, it's complicated.

## Online Sync Service
With what we have right now, it should be ez pez making an online sync service for our app. It would be useful for this scenario:
- A and B have edit access
- A is online from 1pm-2pm every day
- B is online from 3pm-4pm every day

There is never a time when A and B are both online, so they'll never automatically sync. We can fix this by having an always-on server which can store edits.

- A and B have edit access
- A makes edits between 1pm-2pm
- Before disconnecting from the internet, A uploads edits to the server
- At 3pm, B downloads edits from the server
- Before disconnecting, B uploads edits to the server
- The next day, A downloads edits from the server

## Live collaboration
You basically just need a live communication method between online devices, such as [Pusher Channels](https://pusher.com/channels/) (which can easily be replaced with a self-hosted, open source tool). You can just have a channel with all online editors and viewers connected, and when you make an edit you can just publish the commit.

## Making the counter go down
Right now we have a `u64` which can only be `+= 1`d. But what if we want to `-= 1` it too? This can be done really easily
```rs
enum Edit {
  Up
  Down
}

struct Commit {
  id: u64,
  edit: Edit
}
```
