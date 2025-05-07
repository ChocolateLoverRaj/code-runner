This document is not really specific to a custom OS. It is programming-related in general.

References:
- [Merkle-CRDTs Merkle-DAGs meet CRDTs](https://arxiv.org/pdf/2004.00107).
- https://josephg.com/blog/crdts-go-brrr/

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

## Sharing without Viewing
We may want to share edits with anyone which can only be viewed by certain people. For this, we need to encrypt commits when sending them, and give all viewers and editors access to encrypt/decrypt these.
```rs
struct Commits {
  commits: Encrypted<Vec<Commit>>
}
```
This way, people without read access don't even know exactly how many commits there are.

## Removing viewers
We can't stop viewers from viewing commits that they've already received before they were removed as a viewer. But we can stop them from receiving and viewing future commits. The simplest way to do this is to encrypt the data for every viewer.
```rs
struct Data {
  has_r_access: HashSet<Signature>,
  has_w_access: HashSet<Signature>,
  commits: HashSet<Signed<Commit>>
}

struct Commits {
  commits: HashMap<Signature, Encrypted<Vec<Commit>>>
}
```
However, this is obviously not practicle because the size of `Commits` scales by `number of viewers` * `size of commits`.

So we can encrypt the commits with a cipher and only give viewers access to decrypt the cipher
```rs
struct Commits {
  key: HashSet<Signature, AsymmetricEncrypted<Key>>,
  commits: SymmetricEncrypted<Vec<Commit>>
}
```
Now it scales so that the size of `Commits` scales by `number of viewers` * `size of key`.

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
or we could just do
```rs
struct Commit {
  id: u64,
  edit: isize
}
```

## Editing a non-atomic `T`
Let's say we have any type that's not a number, and we don't want to combine edits (such as `+= 1` and `+= 1` resulting in `+= 1`). One examples is a `String` which we don't want to edit simultaneously in multiple places.

The most obvious issue is conflicts:
- A and B are RW
- The string is originally "Greps"
- A changes the string to "Grapes"
- B changes the string to "Green Grapes" before it received edits made by A

Now A and B are not synchronized and there is a conflict if they try to synchronize. Unless we want to show a UI about conflicts, we need an automatic, consistent "merge strategy" (it cannot be "ours" or "theirs" because then A and B would not agree on which one to keep). The easiest merge strategy is just based on date. We can do "first" or "last". This would require putting a timestamp on every commit.

## Editing a HashSet (adding only)
An edit can just be `T`, the value to add to the set. Conflicts can't happen because if the same `T` is added multiple times, then only 1 of it is added because every item has to be unique. We can just ignore duplicates.

## Editing a HashSet (adding and removing)
An edit can just be
```rs
enum EditType {
  Adding
  Removing
}

struct EditData {
  edit_type: EditType,
  data: T
}
```
and now it's possible for conflicts to happen:
- A and B are editors
- A adds `t`
- A removes `t`
- B adds `t`
- Then A and B synchronize with each other

In this case, should `t` be in the set or not? We can do what the Merkle thing does and just use the difference between the number of additions and the number of removes. If the value is added more than it is removed, then it exists in the set.

## Editing an unordered list which can have duplicate items
When adding an item, a random id is created for the item, which is referenced when editing or removing the item.

## Editing an ordered list which can have duplicate items
In this scenario, order matters (which means having some sort of drag-and-drop UI for rearranging). And order is synchronized. The operations we can have:
- Inserting a new item at a certain position
- Editing an existing item
- Removing an existing item
- Moving the position of an existing item

### Multiple inserts in the same position conflict
- A and B are RW
- A inserts an item at index 0
- B inserts an item at index 0

Now which item should be first? We can just do it based on time-stamp.

### Order of commits matters?
Up until now, the order didn't matter. Every state was represented by multiple edits, and changing the order of the edits did not change the final state. For example,
- The number 10 can be made up of +5, +7, -2. The order doesn't matter.
- The set of "Apple", "Pineapple", "Pen" can be made up of adding "Apple", adding "Pineapple", adding "Pen", Adding "Watermelon", and removing "Watermelon". Order doesn't matter.
- A value of "Oats" can be made up of the setting the value to "Oatmeal" at 1pm and then "Oats" at 2pm. Regardless of the order, the newest edit will be in effect.

But now, think about these changes:
- Insert "Potatoes" at index 0
- Insert "Apple" at index 1

Without the potato edit, the list would have zero elements, making an insert at index 1 out of bounds. So the edits must be applied in order.

Or this scenario:
- Insert "Potatoes" at index 0
- Insert "Apple" at index 0

If you made these edits in that order, you would expect the list to be `["Potatoes", "Oranges"]`. But if you apply the commits in reverse order, the list would be `["Oranges", "Potatoes"]`.

Referencing by index can be problematic. Instead, we can do what Yjs and Automerge do and reference the previous element.
```rs
enum Edit {
  Insert {
    after_id: Id,
    id: Id,
    item: T
  },
  Edit {
    id: Id,
    edit: ItemEdit
  },
  Remove {
    id: Id
  }
}
```
Now we can represent `["Potatoes", "Oranges"]` as:
- Insert "Potatoes" (with id: 0) after id `None`
- Insert "Oranges" (with id: 1) after id `Some(1)`

### Removing Items
When we remove the items, we need to still remember the removed item's id because other devices could insert items referencing the deleted item.

### Moving items around
Imagine a grocery list:
- Bananas, quantity 6
- Krave Cereal, quantity 2
- Soy milk, quantity 2

and in the app the user moves soy milk to the top of the list. We can have a move edit type:
```rs
struct MoveEdit {
  item_id: Id,
  new_after_id: Option<Id> // Where `None` means it's first
}
```

### Tuple / `struct`s
This is really easy since there is a fixed amount of items. All we need to do is reference the index of the tuple / struct field and the edit data of that field.

## Nested Data Structures
There is nothing stopping us from having nested data structures. Imagine a collaborative shopping list app (which I want):
- There is an ordered list of items to buy
  - Each item has multiple fields
    - The name is just a cell which holds a string
    - The quantity is a number which can be increased and decreased concurrently
    - There is a `HashSet` of stores which this item can be found in, with each store represented by its id
- There is an unordered list of stores
  - Each store is just a cell which holds a string

Now imagine decreasing the quantity of an item. The edit can be represented as
- Editing the ordered list of items
- Editing a specific item in the list
- Editing the quantity field
- Change the quantity by -1
